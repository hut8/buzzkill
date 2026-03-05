use std::io::BufReader;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Snapshot of the receiver's GPS position.
#[derive(Clone, Debug)]
pub struct GpsFix {
    pub lat: f64,
    pub lon: f64,
    pub alt: f64,
    pub speed: f64,
    pub track: f64,
    pub time: String,
}

/// Shared handle to the latest GPS fix.
pub type GpsHandle = Arc<Mutex<Option<GpsFix>>>;

/// Spawn a background thread that reads from gpsd and updates the shared fix.
/// Returns a handle that can be read from any thread.
pub fn spawn(running: &'static AtomicBool) -> GpsHandle {
    let handle: GpsHandle = Arc::new(Mutex::new(None));
    let h = handle.clone();

    std::thread::Builder::new()
        .name("gps".into())
        .spawn(move || {
            gps_loop(running, &h);
        })
        .expect("failed to spawn GPS thread");

    handle
}

fn gps_loop(running: &AtomicBool, handle: &GpsHandle) {
    let mut backoff = Duration::from_secs(1);
    let max_backoff = Duration::from_secs(30);

    while running.load(Ordering::Relaxed) {
        match TcpStream::connect("127.0.0.1:2947") {
            Ok(stream) => {
                backoff = Duration::from_secs(1);
                log::info!("Connected to gpsd");
                if let Err(e) = read_gpsd(running, handle, stream) {
                    log::warn!("gpsd connection lost: {}", e);
                }
            }
            Err(e) => {
                log::debug!("Cannot connect to gpsd: {} (retrying in {:?})", e, backoff);
            }
        }

        if !running.load(Ordering::Relaxed) {
            break;
        }

        // Clear fix on disconnect
        if let Ok(mut fix) = handle.lock() {
            *fix = None;
        }

        std::thread::sleep(backoff);
        backoff = (backoff * 2).min(max_backoff);
    }

    log::info!("GPS thread stopping");
}

fn read_gpsd(
    running: &AtomicBool,
    handle: &GpsHandle,
    stream: TcpStream,
) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    let reader_stream = stream.try_clone().map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(reader_stream);
    let mut writer = stream;

    gpsd_proto::handshake(&mut reader, &mut writer)
        .map_err(|e| format!("handshake failed: {:?}", e))?;
    log::info!("gpsd handshake complete");

    while running.load(Ordering::Relaxed) {
        match gpsd_proto::get_data(&mut reader) {
            Ok(gpsd_proto::ResponseData::Tpv(tpv)) => {
                if let (Some(lat), Some(lon)) = (tpv.lat, tpv.lon) {
                    let fix = GpsFix {
                        lat,
                        lon,
                        alt: tpv.alt.unwrap_or(0.0) as f64,
                        speed: tpv.speed.unwrap_or(0.0) as f64,
                        track: tpv.track.unwrap_or(0.0) as f64,
                        time: tpv.time.unwrap_or_default(),
                    };
                    log::debug!(
                        "GPS fix: lat={:.7} lon={:.7} alt={:.1}m",
                        fix.lat,
                        fix.lon,
                        fix.alt
                    );
                    if let Ok(mut guard) = handle.lock() {
                        *guard = Some(fix);
                    }
                }
            }
            Ok(_) => {} // Sky, Device, etc — ignore
            Err(gpsd_proto::GpsdError::IoError(ref e))
                if e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::WouldBlock =>
            {
                continue;
            }
            Err(e) => return Err(format!("gpsd error: {:?}", e)),
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Geo helpers
// ---------------------------------------------------------------------------

const EARTH_RADIUS_M: f64 = 6_371_000.0;

/// Haversine distance between two lat/lon points in meters.
pub fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_M * a.sqrt().asin()
}

/// Absolute bearing from point 1 to point 2, in degrees (0 = N, clockwise).
pub fn bearing(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let lat1 = lat1.to_radians();
    let lat2 = lat2.to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let x = d_lon.sin() * lat2.cos();
    let y = lat1.cos() * lat2.sin() - lat1.sin() * lat2.cos() * d_lon.cos();
    (x.atan2(y).to_degrees() + 360.0) % 360.0
}

/// Relative bearing given our heading (track) and an absolute bearing.
/// Returns -180..+180 or 0..360 — we return 0..360 for clock-position use.
pub fn relative_bearing(our_track: f64, absolute_bearing: f64) -> f64 {
    ((absolute_bearing - our_track) % 360.0 + 360.0) % 360.0
}

/// Convert relative bearing (0..360) to clock position string, e.g. "12:00", "3:00".
pub fn clock_position(relative_deg: f64) -> String {
    let hour = ((relative_deg / 30.0).round() as u32) % 12;
    let hour = if hour == 0 { 12 } else { hour };
    format!("{}:00", hour)
}

/// Convert an absolute bearing to a compass direction.
pub fn compass_direction(brg: f64) -> &'static str {
    let brg = ((brg % 360.0) + 360.0) % 360.0;
    match brg as u32 {
        0..=22 => "N",
        23..=67 => "NE",
        68..=112 => "E",
        113..=157 => "SE",
        158..=202 => "S",
        203..=247 => "SW",
        248..=292 => "W",
        293..=337 => "NW",
        _ => "N",
    }
}

/// Format distance for display: meters if < 1000, km otherwise.
pub fn format_distance(meters: f64) -> String {
    if meters < 1000.0 {
        format!("{:.0}m", meters)
    } else {
        format!("{:.1}km", meters / 1000.0)
    }
}

