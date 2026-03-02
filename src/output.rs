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
pub fn print_new_drone(transport: &str, mac: &[u8; 6], rssi: i8) {
    println!(
        "[+] NEW DRONE  [{}] mac={} rssi={}dBm",
        transport,
        format_mac(mac),
        rssi,
    );
}

/// Print a decoded message update.
pub fn print_message(transport: &str, mac: &[u8; 6], rssi: i8, msg: &DroneIdMessage) {
    let mac_str = format_mac(mac);
    match msg {
        DroneIdMessage::BasicId(bid) => {
            println!(
                "  [BasicID]    [{}] mac={} id_type={} ua_type={} ua_id=\"{}\"",
                transport, mac_str, bid.id_type, bid.ua_type, bid.ua_id
            );
        }
        DroneIdMessage::Location(loc) => {
            println!(
                "  [Location]   [{}] mac={} lat={:.7} lon={:.7} alt_p={:.1}m alt_g={:.1}m height={:.1}m speed={:.1}m/s dir={:.0} rssi={}dBm",
                transport,
                mac_str,
                loc.latitude,
                loc.longitude,
                loc.altitude_pressure,
                loc.altitude_geodetic,
                loc.height_above_takeoff,
                loc.speed_horizontal,
                loc.direction,
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
        DroneIdMessage::Unknown { msg_type, .. } => {
            log::debug!(
                "  [Unknown]    [{}] mac={} msg_type=0x{:X}",
                transport,
                mac_str,
                msg_type
            );
        }
    }
}

/// Print drone lost contact.
pub fn print_lost(mac: &[u8; 6], state: &DroneState) {
    let id = state
        .basic_id
        .as_ref()
        .map(|b| b.ua_id.as_str())
        .unwrap_or("unknown");
    println!(
        "[-] LOST DRONE [{}] mac={} id=\"{}\" msgs={}",
        state.transport,
        format_mac(mac),
        id,
        state.msg_count,
    );
}
