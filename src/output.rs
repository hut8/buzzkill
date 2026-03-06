use crate::gps::{self, GpsFix};
use crate::remoteid::decode::DroneIdMessage;
use crate::tracker::DroneState;

/// Format a MAC address as a colon-separated hex string.
pub fn format_mac(mac: &[u8; 6]) -> String {
    format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    )
}

/// Print a new drone discovery.
pub fn print_new_drone(
    transport: &str,
    mac: &[u8; 6],
    rssi: i8,
    addr_type: Option<u8>,
    drone_loc: Option<(f64, f64)>,
    gps_fix: Option<&GpsFix>,
) {
    let addr_kind = match addr_type {
        Some(0) => " (public)".to_string(),
        Some(1) => " (random)".to_string(),
        Some(2) => " (public identity)".to_string(),
        Some(3) => " (random identity)".to_string(),
        Some(other) => format!(" (addr_type:{})", other),
        None => String::new(),
    };
    let geo_info = match drone_loc {
        Some((lat, lon)) => format_geo_info(lat, lon, gps_fix),
        None => String::new(),
    };
    println!(
        "[+] NEW DRONE  [{}] mac={}{} rssi={}dBm{}",
        transport,
        format_mac(mac),
        addr_kind,
        rssi,
        geo_info,
    );
}

/// Print a decoded message update.
pub fn print_message(
    transport: &str,
    mac: &[u8; 6],
    rssi: i8,
    msg: &DroneIdMessage,
    gps_fix: Option<&GpsFix>,
) {
    let mac_str = format_mac(mac);
    match msg {
        DroneIdMessage::BasicId(bid) => {
            println!(
                "  [BasicID]    [{}] mac={} id_type={} ua_type={} ua_id=\"{}\"",
                transport, mac_str, bid.id_type, bid.ua_type, bid.ua_id
            );
        }
        DroneIdMessage::Location(loc) => {
            let geo_info = format_geo_info(loc.latitude, loc.longitude, gps_fix);
            println!(
                "  [Location]   [{}] mac={} lat={:.7} lon={:.7} alt_p={:.1}m alt_g={:.1}m height={:.1}m speed={:.1}m/s dir={:.0}{} rssi={}dBm",
                transport,
                mac_str,
                loc.latitude,
                loc.longitude,
                loc.altitude_pressure,
                loc.altitude_geodetic,
                loc.height_above_takeoff,
                loc.speed_horizontal,
                loc.direction,
                geo_info,
                rssi,
            );
        }
        DroneIdMessage::System(sys) => {
            println!(
                "  [System]     [{}] mac={} op_lat={:.7} op_lon={:.7} area_count={} area_radius={}m",
                transport,
                mac_str,
                sys.operator_latitude,
                sys.operator_longitude,
                sys.area_count,
                sys.area_radius,
            );
        }
        DroneIdMessage::OperatorId(oid) => {
            println!(
                "  [OperatorID] [{}] mac={} operator_id=\"{}\"",
                transport, mac_str, oid.operator_id
            );
        }
        DroneIdMessage::SelfId(sid) => {
            println!(
                "  [SelfID]     [{}] mac={} desc=\"{}\"",
                transport, mac_str, sid.description
            );
        }
        DroneIdMessage::Auth(auth) => {
            println!(
                "  [Auth]       [{}] mac={} page={}/{}",
                transport, mac_str, auth.page_number, auth.page_count
            );
        }
        DroneIdMessage::Unknown {
            msg_type,
            proto_version,
        } => {
            log::debug!(
                "  [Unknown]    [{}] mac={} msg_type=0x{:X} proto_version={}",
                transport,
                mac_str,
                msg_type,
                proto_version
            );
        }
    }
}

/// Format geo info (distance/bearing) when we have a GPS fix and drone location.
fn format_geo_info(drone_lat: f64, drone_lon: f64, gps_fix: Option<&GpsFix>) -> String {
    // Skip invalid coordinates (0,0 means no fix from drone)
    if drone_lat.abs() < 0.0001 && drone_lon.abs() < 0.0001 {
        return String::new();
    }
    let fix = match gps_fix {
        Some(f) => f,
        None => return String::new(),
    };
    let dist = gps::haversine_distance(fix.lat, fix.lon, drone_lat, drone_lon);
    let abs_brg = gps::bearing(fix.lat, fix.lon, drone_lat, drone_lon);
    let compass = gps::compass_direction(abs_brg);
    let rel = gps::relative_bearing(fix.track, abs_brg);
    let clock = gps::clock_position(rel);
    let normalized_bearing = (abs_brg.round() as u32) % 360;
    format!(
        " dist={} bearing={}°({}) clock={}",
        gps::format_distance(dist),
        normalized_bearing,
        compass,
        clock,
    )
}

/// Print drone lost contact.
pub fn print_lost(state: &DroneState) {
    let id = state
        .basic_id
        .as_ref()
        .map(|b| b.ua_id.as_str())
        .unwrap_or("unknown");
    let duration = state.last_seen.duration_since(state.first_seen);
    println!(
        "[-] LOST DRONE [{}] mac={} id=\"{}\" msgs={} tracked={:.0}s",
        state.transport,
        format_mac(&state.mac),
        id,
        state.msg_count,
        duration.as_secs_f64(),
    );
}
