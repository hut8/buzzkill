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
    #[allow(dead_code)]
    pub speed: f64,
    pub track: f64,
    #[allow(dead_code)]
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

fn read_gpsd(running: &AtomicBool, handle: &GpsHandle, stream: TcpStream) -> Result<(), String> {
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
                } else if let Ok(mut guard) = handle.lock() {
                    *guard = None;
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
    let mut a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
    a = a.clamp(0.0, 1.0);
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
pub fn relative_bearing(our_track: f64, absolute_bearing: f64) -> f64 {
    ((absolute_bearing - our_track) % 360.0 + 360.0) % 360.0
}

/// Convert relative bearing (0..360) to clock position string.
pub fn clock_position(relative_deg: f64) -> String {
    let hour = ((relative_deg / 30.0).round() as u32) % 12;
    let hour = if hour == 0 { 12 } else { hour };
    format!("{}:00", hour)
}

/// Convert an absolute bearing to a compass direction.
pub fn compass_direction(brg: f64) -> &'static str {
    let brg = ((brg % 360.0) + 360.0) % 360.0;
    let octant = ((brg / 45.0).round() as u32) % 8;
    match octant {
        0 => "N",
        1 => "NE",
        2 => "E",
        3 => "SE",
        4 => "S",
        5 => "SW",
        6 => "W",
        7 => "NW",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn haversine_known_distance() {
        let dist = haversine_distance(40.7128, -74.0060, 34.0522, -118.2437);
        assert!((dist - 3_944_000.0).abs() < 50_000.0, "dist={}", dist);
    }

    #[test]
    fn haversine_same_point() {
        let dist = haversine_distance(51.5, -0.1, 51.5, -0.1);
        assert!(dist < 0.01, "dist={}", dist);
    }

    #[test]
    fn bearing_due_north() {
        let brg = bearing(51.0, 0.0, 52.0, 0.0);
        assert!(brg < 1.0 || brg > 359.0, "brg={}", brg);
    }

    #[test]
    fn bearing_due_east() {
        let brg = bearing(0.0, 0.0, 0.0, 1.0);
        assert!((brg - 90.0).abs() < 1.0, "brg={}", brg);
    }

    #[test]
    fn bearing_due_south() {
        let brg = bearing(52.0, 0.0, 51.0, 0.0);
        assert!((brg - 180.0).abs() < 1.0, "brg={}", brg);
    }

    #[test]
    fn relative_bearing_ahead() {
        assert!((relative_bearing(90.0, 90.0) - 0.0).abs() < 0.01);
    }

    #[test]
    fn relative_bearing_right() {
        assert!((relative_bearing(0.0, 90.0) - 90.0).abs() < 0.01);
    }

    #[test]
    fn relative_bearing_behind() {
        assert!((relative_bearing(0.0, 180.0) - 180.0).abs() < 0.01);
    }

    #[test]
    fn clock_position_values() {
        assert_eq!(clock_position(0.0), "12:00");
        assert_eq!(clock_position(90.0), "3:00");
        assert_eq!(clock_position(180.0), "6:00");
        assert_eq!(clock_position(270.0), "9:00");
        assert_eq!(clock_position(30.0), "1:00");
        assert_eq!(clock_position(345.0), "12:00");
        assert_eq!(clock_position(330.0), "11:00");
    }

    #[test]
    fn compass_direction_all_octants() {
        assert_eq!(compass_direction(0.0), "N");
        assert_eq!(compass_direction(45.0), "NE");
        assert_eq!(compass_direction(90.0), "E");
        assert_eq!(compass_direction(135.0), "SE");
        assert_eq!(compass_direction(180.0), "S");
        assert_eq!(compass_direction(225.0), "SW");
        assert_eq!(compass_direction(270.0), "W");
        assert_eq!(compass_direction(315.0), "NW");
        assert_eq!(compass_direction(359.0), "N");
    }

    #[test]
    fn format_distance_meters_and_km() {
        assert_eq!(format_distance(500.0), "500m");
        assert_eq!(format_distance(1500.0), "1.5km");
        assert_eq!(format_distance(50.0), "50m");
    }
}
