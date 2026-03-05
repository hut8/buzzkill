pub mod frames;
pub mod socket;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::db::{self, SightingRow};
use crate::email;
use crate::gps::GpsHandle;
use crate::output;
use crate::remoteid::decode;
use crate::tracker::Tracker;

use self::frames::parse_remote_id_beacon;
use self::socket::WifiMonSocket;

pub fn run(
    iface: &str,
    running: &'static AtomicBool,
    db_tx: Option<mpsc::SyncSender<SightingRow>>,
    email_tx: Option<mpsc::SyncSender<email::DroneAlert>>,
    tracker: Arc<Mutex<Tracker>>,
    gps: GpsHandle,
) {
    let sock = match WifiMonSocket::open(iface) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to open WiFi monitor socket on {}: {}", iface, e);
            return;
        }
    };

    log::info!("WiFi scanner running on {}", iface);

    let mut buf = [0u8; 4096];
    let mut last_expire = Instant::now();

    while running.load(Ordering::Relaxed) {
        let n = match sock.read_frame(&mut buf) {
            Ok(n) => n,
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => {
                log::error!("WiFi read error: {}", e);
                break;
            }
        };

        if n == 0 {
            continue;
        }

        let frame = &buf[..n];

        if let Some(beacon) = parse_remote_id_beacon(frame) {
            let messages = decode::decode_all(&beacon.message);
            let gps_fix = gps.lock().ok().and_then(|g| g.clone());
            for msg in &messages {
                let is_new = {
                    let mut t = tracker.lock().unwrap();
                    t.update(&beacon.mac, beacon.rssi, beacon.counter, msg, "wifi")
                };

                if is_new {
                    let drone_loc = {
                        let t = tracker.lock().unwrap();
                        t.drones
                            .get(&beacon.mac)
                            .and_then(|d| d.location.as_ref())
                            .map(|l| (l.latitude, l.longitude))
                    };
                    output::print_new_drone(
                        "wifi",
                        &beacon.mac,
                        beacon.rssi,
                        None,
                        drone_loc,
                        gps_fix.as_ref(),
                    );
                    if let Some(ref tx) = email_tx {
                        if let Err(e) = tx.try_send(email::DroneAlert {
                            transport: "wifi",
                            mac: beacon.mac,
                            rssi: beacon.rssi,
                        }) {
                            log::error!("Email notification dropped: {}", e);
                        }
                    }
                }
                output::print_message("wifi", &beacon.mac, beacon.rssi, msg, gps_fix.as_ref());

                if let Some(ref tx) = db_tx {
                    let row = db::build_row(
                        "wifi",
                        &beacon.mac,
                        beacon.rssi,
                        beacon.counter,
                        msg,
                        gps_fix.as_ref(),
                    );
                    let _ = tx.try_send(row);
                }
            }
        }

        if last_expire.elapsed().as_secs() >= 5 {
            tracker.lock().unwrap().expire();
            last_expire = Instant::now();
        }
    }

    log::info!("WiFi scanner stopped");
}
