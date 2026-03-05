mod db;
mod gps;
mod hci;
mod output;
mod remoteid;
mod tracker;
mod wifi;

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Instant;

use crate::hci::commands;
use crate::hci::events;
use crate::hci::socket::HciSocket;
use crate::remoteid::decode;
use crate::remoteid::filter;
use crate::tracker::Tracker;

fn parse_adapter_index(name: &str) -> Option<u16> {
    name.strip_prefix("hci").and_then(|s| s.parse().ok())
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = std::env::args().collect();
    let adapter = args.get(1).map(|s| s.as_str()).unwrap_or("hci0");
    let dev_id = parse_adapter_index(adapter).unwrap_or_else(|| {
        eprintln!("Invalid adapter name: {} (expected hciN)", adapter);
        std::process::exit(1);
    });

    let expiry_secs: u64 = match args.iter().position(|a| a == "--expiry") {
        Some(i) => match args.get(i + 1) {
            Some(val) => val.parse().unwrap_or_else(|_| {
                eprintln!("Invalid value for --expiry: {}", val);
                std::process::exit(1);
            }),
            None => {
                eprintln!("Missing value for --expiry");
                std::process::exit(1);
            }
        },
        None => 60,
    };

    let wifi_iface: Option<String> = match args.iter().position(|a| a == "--wifi") {
        Some(i) => match args.get(i + 1) {
            Some(val) => Some(val.clone()),
            None => {
                eprintln!("Missing value for --wifi");
                std::process::exit(1);
            }
        },
        None => None,
    };

    // Set up DB channel if DATABASE_URL is set
    let db_tx: Option<mpsc::SyncSender<db::SightingRow>> = if std::env::var("DATABASE_URL").is_ok()
    {
        let (tx, rx) = mpsc::sync_channel::<db::SightingRow>(1000);
        db::spawn_writer(rx);
        log::info!("Database logging enabled");
        Some(tx)
    } else {
        log::warn!("DATABASE_URL not set — database logging disabled");
        None
    };

    log::info!("Opening HCI socket on {} (dev_id={})", adapter, dev_id);
    let sock = HciSocket::open(dev_id).unwrap_or_else(|e| {
        eprintln!(
            "Failed to open HCI socket: {} (try running with sudo or CAP_NET_RAW)",
            e
        );
        std::process::exit(1);
    });

    log::info!("Configuring LE scan parameters (passive, 100ms interval/window)");
    if let Err(e) = commands::le_set_scan_parameters(&sock) {
        eprintln!("Failed to set scan parameters: {}", e);
        std::process::exit(1);
    }

    log::info!("Enabling LE scan");
    if let Err(e) = commands::le_set_scan_enable(&sock, true) {
        eprintln!("Failed to enable scan: {}", e);
        std::process::exit(1);
    }

    // Handle Ctrl+C via a static AtomicBool (signal-safe)
    unsafe {
        libc::signal(
            libc::SIGINT,
            signal_handler as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGTERM,
            signal_handler as *const () as libc::sighandler_t,
        );
    }

    // Start GPS reader
    let gps = gps::spawn(&RUNNING);

    // Spawn WiFi scanner thread if requested
    if let Some(iface) = wifi_iface {
        let wifi_db_tx = db_tx.clone();
        let wifi_gps = gps.clone();
        std::thread::spawn(move || {
            wifi::run(&iface, &RUNNING, wifi_db_tx, expiry_secs, wifi_gps);
        });
    }

    println!(
        "Buzzkill scanning for Remote ID drones on {}... (Ctrl+C to stop)",
        adapter
    );

    let mut tracker = Tracker::new(expiry_secs);
    let mut buf = [0u8; 1024];
    let mut last_expire = Instant::now();

    while RUNNING.load(Ordering::Relaxed) {
        let n = match sock.read_event(&mut buf) {
            Ok(n) => n,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => {
                log::error!("Read error: {}", e);
                break;
            }
        };

        if n < 2 {
            continue;
        }

        // buf[0] is the HCI packet indicator (0x04 for events), rest is the event
        let event_buf = if buf[0] == 0x04 {
            &buf[1..n]
        } else {
            &buf[..n]
        };

        if let Some(reports) = events::parse_hci_event(event_buf) {
            for report in &reports {
                let payloads = filter::extract_remote_id(&report.data);
                for payload in &payloads {
                    log::debug!(
                        "Remote ID payload from {} event_type=0x{:02X}",
                        output::format_mac(&report.addr),
                        report.event_type
                    );
                    let messages = decode::decode_all(&payload.message);
                    let gps_fix = gps.lock().ok().and_then(|g| g.clone());
                    for msg in &messages {
                        let is_new =
                            tracker.update(&report.addr, report.rssi, payload.counter, msg, "ble");

                        if is_new {
                            let drone_loc = tracker
                                .drones
                                .get(&report.addr)
                                .and_then(|d| d.location.as_ref())
                                .map(|l| (l.latitude, l.longitude));
                            output::print_new_drone(
                                "ble",
                                &report.addr,
                                report.rssi,
                                Some(report.addr_type),
                                drone_loc,
                                &gps,
                            );
                        }
                        output::print_message("ble", &report.addr, report.rssi, msg, &gps);

                        if let Some(ref tx) = db_tx {
                            let row = db::build_row(
                                "ble",
                                &report.addr,
                                report.rssi,
                                payload.counter,
                                msg,
                                gps_fix.as_ref(),
                            );
                            let _ = tx.try_send(row);
                        }
                    }
                }
            }
        }

        // Periodic expiry check
        if last_expire.elapsed().as_secs() >= 5 {
            tracker.expire();
            last_expire = Instant::now();
        }
    }

    log::info!("Disabling LE scan");
    let _ = commands::le_set_scan_enable(&sock, false);
    println!(
        "Scan stopped. Tracked {} drone(s) total.",
        tracker.drones.len()
    );
}

static RUNNING: AtomicBool = AtomicBool::new(true);

extern "C" fn signal_handler(_sig: libc::c_int) {
    RUNNING.store(false, Ordering::SeqCst);
}
