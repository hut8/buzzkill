use std::fmt;

/// OpenDroneID message type identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    BasicId,
    Location,
    Auth,
    SelfId,
    System,
    OperatorId,
    MessagePack,
    Unknown(u8),
}

impl MessageType {
    pub fn from_nibble(val: u8) -> Self {
        match val {
            0x0 => Self::BasicId,
            0x1 => Self::Location,
            0x2 => Self::Auth,
            0x3 => Self::SelfId,
            0x4 => Self::System,
            0x5 => Self::OperatorId,
            0xF => Self::MessagePack,
            other => Self::Unknown(other),
        }
    }
}

impl fmt::Display for MessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BasicId => write!(f, "BasicID"),
            Self::Location => write!(f, "Location"),
            Self::Auth => write!(f, "Auth"),
            Self::SelfId => write!(f, "SelfID"),
            Self::System => write!(f, "System"),
            Self::OperatorId => write!(f, "OperatorID"),
            Self::MessagePack => write!(f, "MessagePack"),
            Self::Unknown(v) => write!(f, "Unknown(0x{:X})", v),
        }
    }
}

/// ID Type for Basic ID message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdType {
    None,
    SerialNumber,
    CaaRegistration,
    UtmAssigned,
    SpecificSession,
    Unknown(u8),
}

impl IdType {
    fn from_val(val: u8) -> Self {
        match val {
            0 => Self::None,
            1 => Self::SerialNumber,
            2 => Self::CaaRegistration,
            3 => Self::UtmAssigned,
            4 => Self::SpecificSession,
            other => Self::Unknown(other),
        }
    }
}

impl fmt::Display for IdType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::SerialNumber => write!(f, "SerialNumber"),
            Self::CaaRegistration => write!(f, "CAARegistration"),
            Self::UtmAssigned => write!(f, "UTMAssigned"),
            Self::SpecificSession => write!(f, "SpecificSession"),
            Self::Unknown(v) => write!(f, "Unknown({})", v),
        }
    }
}

/// UA (Unmanned Aircraft) type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UaType {
    None,
    Aeroplane,
    HelicopterOrMultirotor,
    Gyroplane,
    HybridLift,
    Ornithopter,
    Glider,
    Kite,
    FreeBalloon,
    CaptiveBalloon,
    Airship,
    FreeFallParachute,
    Rocket,
    TetheredPoweredAircraft,
    GroundObstacle,
    Other,
    Unknown(u8),
}

impl UaType {
    fn from_val(val: u8) -> Self {
        match val {
            0 => Self::None,
            1 => Self::Aeroplane,
            2 => Self::HelicopterOrMultirotor,
            3 => Self::Gyroplane,
            4 => Self::HybridLift,
            5 => Self::Ornithopter,
            6 => Self::Glider,
            7 => Self::Kite,
            8 => Self::FreeBalloon,
            9 => Self::CaptiveBalloon,
            10 => Self::Airship,
            11 => Self::FreeFallParachute,
            12 => Self::Rocket,
            13 => Self::TetheredPoweredAircraft,
            14 => Self::GroundObstacle,
            15 => Self::Other,
            other => Self::Unknown(other),
        }
    }
}

impl fmt::Display for UaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// Decoded Basic ID message.
#[derive(Debug, Clone)]
pub struct BasicId {
    pub id_type: IdType,
    pub ua_type: UaType,
    pub ua_id: String,
}

/// Decoded Location/Vector message.
#[derive(Debug, Clone)]
pub struct Location {
    pub status: u8,
    pub direction: f64,            // degrees
    pub speed_horizontal: f64,     // m/s
    pub speed_vertical: f64,       // m/s
    pub latitude: f64,             // degrees
    pub longitude: f64,            // degrees
    pub altitude_pressure: f64,    // meters (WGS84)
    pub altitude_geodetic: f64,    // meters
    pub height_above_takeoff: f64, // meters
    pub timestamp: f64,            // seconds since the hour, 0.1s resolution
}

/// Decoded System message.
#[derive(Debug, Clone)]
pub struct System {
    pub operator_latitude: f64,
    pub operator_longitude: f64,
    pub area_count: u16,
    pub area_radius: u16, // meters
    pub area_ceiling: f64,
    pub area_floor: f64,
    pub classification_type: u8,
    pub operator_altitude_geo: f64,
}

/// Decoded Operator ID message.
#[derive(Debug, Clone)]
pub struct OperatorId {
    pub operator_id_type: u8,
    pub operator_id: String,
}

/// Decoded Self-ID message.
#[derive(Debug, Clone)]
pub struct SelfId {
    pub description_type: u8,
    pub description: String,
}

/// Decoded Auth message (partial — stores raw page data).
#[derive(Debug, Clone)]
pub struct Auth {
    pub auth_type: u8,
    pub page_number: u8,
    pub page_count: u8,
    pub length: u8,
    pub timestamp: u32,
    pub data: Vec<u8>,
}

/// A decoded OpenDroneID message.
#[derive(Debug, Clone)]
pub enum DroneIdMessage {
    BasicId(BasicId),
    Location(Location),
    Auth(Auth),
    SelfId(SelfId),
    System(System),
    OperatorId(OperatorId),
    Unknown { msg_type: u8, proto_version: u8 },
}

impl DroneIdMessage {
    pub fn msg_type(&self) -> MessageType {
        match self {
            Self::BasicId(_) => MessageType::BasicId,
            Self::Location(_) => MessageType::Location,
            Self::Auth(_) => MessageType::Auth,
            Self::SelfId(_) => MessageType::SelfId,
            Self::System(_) => MessageType::System,
            Self::OperatorId(_) => MessageType::OperatorId,
            Self::Unknown { msg_type, .. } => MessageType::from_nibble(*msg_type),
        }
    }
}

/// Decode a 25-byte OpenDroneID message.
pub fn decode_message(data: &[u8; 25]) -> DroneIdMessage {
    let header = data[0];
    let msg_type_nibble = (header >> 4) & 0x0F;
    let proto_version = header & 0x0F;

    match MessageType::from_nibble(msg_type_nibble) {
        MessageType::BasicId => decode_basic_id(data),
        MessageType::Location => decode_location(data),
        MessageType::Auth => decode_auth(data),
        MessageType::SelfId => decode_self_id(data),
        MessageType::System => decode_system(data),
        MessageType::OperatorId => decode_operator_id(data),
        _ => DroneIdMessage::Unknown {
            msg_type: msg_type_nibble,
            proto_version,
        },
    }
}

/// Decode multiple messages from a single 25-byte payload if it's a Message Pack,
/// otherwise decode the single message. Returns a vec of decoded messages.
pub fn decode_all(data: &[u8; 25]) -> Vec<DroneIdMessage> {
    if MessageType::from_nibble((data[0] >> 4) & 0x0F) == MessageType::MessagePack {
        decode_message_pack(data)
    } else {
        vec![decode_message(data)]
    }
}

fn decode_basic_id(data: &[u8; 25]) -> DroneIdMessage {
    let id_type = IdType::from_val((data[1] >> 4) & 0x0F);
    let ua_type = UaType::from_val(data[1] & 0x0F);

    // UA ID is bytes 2..22 (20 bytes), null-terminated ASCII
    let ua_id = extract_ascii_string(&data[2..22]);

    DroneIdMessage::BasicId(BasicId {
        id_type,
        ua_type,
        ua_id,
    })
}

fn decode_location(data: &[u8; 25]) -> DroneIdMessage {
    let status = (data[1] >> 4) & 0x0F;
    let _height_type = (data[1] >> 2) & 0x01;
    let _ew_direction = (data[1] >> 1) & 0x01;
    let speed_multiplier = data[1] & 0x01;

    let direction = {
        let raw = data[2] as f64;
        if data[1] & 0x02 != 0 {
            raw + 180.0
        } else {
            raw
        }
    };

    let speed_horizontal = {
        let raw = data[3] as f64;
        if speed_multiplier == 0 {
            raw * 0.25 // 0.25 m/s resolution
        } else {
            (raw * 0.75) + (255.0 * 0.25) // extended range
        }
    };

    let speed_vertical = {
        let raw = data[4] as i8;
        raw as f64 * 0.5
    };

    let latitude = i32::from_le_bytes([data[5], data[6], data[7], data[8]]) as f64 * 1e-7;
    let longitude = i32::from_le_bytes([data[9], data[10], data[11], data[12]]) as f64 * 1e-7;

    let altitude_pressure = u16::from_le_bytes([data[13], data[14]]) as f64 * 0.5 - 1000.0;
    let altitude_geodetic = u16::from_le_bytes([data[15], data[16]]) as f64 * 0.5 - 1000.0;

    let height_above_takeoff = u16::from_le_bytes([data[17], data[18]]) as f64 * 0.5 - 1000.0;

    // Timestamp: 0.1s resolution since the hour
    let timestamp = u16::from_le_bytes([data[21], data[22]]) as f64 * 0.1;

    DroneIdMessage::Location(Location {
        status,
        direction,
        speed_horizontal,
        speed_vertical,
        latitude,
        longitude,
        altitude_pressure,
        altitude_geodetic,
        height_above_takeoff,
        timestamp,
    })
}

fn decode_auth(data: &[u8; 25]) -> DroneIdMessage {
    let auth_type = (data[1] >> 4) & 0x0F;
    let page_number = data[1] & 0x0F;

    if page_number == 0 {
        let page_count = data[2];
        let length = data[3];
        let timestamp = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let auth_data = data[8..25].to_vec();
        DroneIdMessage::Auth(Auth {
            auth_type,
            page_number,
            page_count,
            length,
            timestamp,
            data: auth_data,
        })
    } else {
        let auth_data = data[2..25].to_vec();
        DroneIdMessage::Auth(Auth {
            auth_type,
            page_number,
            page_count: 0,
            length: 0,
            timestamp: 0,
            data: auth_data,
        })
    }
}

fn decode_self_id(data: &[u8; 25]) -> DroneIdMessage {
    let description_type = data[1];
    let description = extract_ascii_string(&data[2..25]);
    DroneIdMessage::SelfId(SelfId {
        description_type,
        description,
    })
}

fn decode_system(data: &[u8; 25]) -> DroneIdMessage {
    let classification_type = (data[1] >> 4) & 0x0F;

    let operator_latitude = i32::from_le_bytes([data[2], data[3], data[4], data[5]]) as f64 * 1e-7;
    let operator_longitude = i32::from_le_bytes([data[6], data[7], data[8], data[9]]) as f64 * 1e-7;

    let area_count = u16::from_le_bytes([data[10], data[11]]);
    let area_radius = data[12] as u16 * 10; // 10m resolution
    let area_ceiling = u16::from_le_bytes([data[13], data[14]]) as f64 * 0.5 - 1000.0;
    let area_floor = u16::from_le_bytes([data[15], data[16]]) as f64 * 0.5 - 1000.0;

    let operator_altitude_geo = u16::from_le_bytes([data[21], data[22]]) as f64 * 0.5 - 1000.0;

    DroneIdMessage::System(System {
        operator_latitude,
        operator_longitude,
        area_count,
        area_radius,
        area_ceiling,
        area_floor,
        classification_type,
        operator_altitude_geo,
    })
}

fn decode_operator_id(data: &[u8; 25]) -> DroneIdMessage {
    let operator_id_type = data[1];
    let operator_id = extract_ascii_string(&data[2..22]);

    DroneIdMessage::OperatorId(OperatorId {
        operator_id_type,
        operator_id,
    })
}

fn decode_message_pack(data: &[u8; 25]) -> Vec<DroneIdMessage> {
    // Message Pack: byte 1 = number of messages, byte 2+ = packed 25-byte messages
    // In BLE single advertisements, space is very limited so message packs
    // are primarily seen in BT5 long range or WiFi. For a standard BLE ad
    // the 25-byte payload can't hold multiple 25-byte sub-messages, so this
    // is a best-effort parse.
    let _num_messages = data[1];
    // Not enough room in a single 25-byte frame for sub-messages.
    // Return as unknown for now.
    vec![DroneIdMessage::Unknown {
        msg_type: 0xF,
        proto_version: data[0] & 0x0F,
    }]
}

fn extract_ascii_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_basic_id_serial() {
        let mut msg = [0u8; 25];
        msg[0] = 0x02; // type=0 (BasicID), version=2
        msg[1] = 0x12; // id_type=1 (serial), ua_type=2 (helicopter/multirotor)
        let serial = b"1234567890ABCDEFGHIJ";
        msg[2..22].copy_from_slice(serial);

        match decode_message(&msg) {
            DroneIdMessage::BasicId(bid) => {
                assert_eq!(bid.id_type, IdType::SerialNumber);
                assert_eq!(bid.ua_type, UaType::HelicopterOrMultirotor);
                assert_eq!(bid.ua_id, "1234567890ABCDEFGHIJ");
            }
            other => panic!("Expected BasicId, got {:?}", other),
        }
    }

    #[test]
    fn test_decode_location() {
        let mut msg = [0u8; 25];
        msg[0] = 0x12; // type=1 (Location), version=2
        msg[1] = 0x20; // status=2 (airborne), height_type=0, ew=0, speed_mult=0

        // direction
        msg[2] = 90; // 90 degrees

        // horizontal speed: 40 * 0.25 = 10.0 m/s
        msg[3] = 40;

        // vertical speed: 4 * 0.5 = 2.0 m/s
        msg[4] = 4;

        // latitude: 47.3977 degrees => 473977000 = 0x1C3E7548
        let lat: i32 = 473977000;
        msg[5..9].copy_from_slice(&lat.to_le_bytes());

        // longitude: 8.5456 degrees => 85456000 = 0x05180100
        let lon: i32 = 85456000;
        msg[9..13].copy_from_slice(&lon.to_le_bytes());

        // altitude_pressure: 500m => (500 + 1000) / 0.5 = 3000
        let alt: u16 = 3000;
        msg[13..15].copy_from_slice(&alt.to_le_bytes());

        // altitude_geodetic: same
        msg[15..17].copy_from_slice(&alt.to_le_bytes());

        // height: 100m => (100 + 1000) / 0.5 = 2200
        let height: u16 = 2200;
        msg[17..19].copy_from_slice(&height.to_le_bytes());

        // timestamp: 1234 * 0.1 = 123.4s
        let ts: u16 = 1234;
        msg[21..23].copy_from_slice(&ts.to_le_bytes());

        match decode_message(&msg) {
            DroneIdMessage::Location(loc) => {
                assert!((loc.latitude - 47.3977).abs() < 0.0001);
                assert!((loc.longitude - 8.5456).abs() < 0.0001);
                assert!((loc.altitude_pressure - 500.0).abs() < 0.1);
                assert!((loc.height_above_takeoff - 100.0).abs() < 0.1);
                assert!((loc.speed_horizontal - 10.0).abs() < 0.1);
                assert!((loc.speed_vertical - 2.0).abs() < 0.1);
                assert_eq!(loc.direction, 90.0);
                assert!((loc.timestamp - 123.4).abs() < 0.1);
            }
            other => panic!("Expected Location, got {:?}", other),
        }
    }

    #[test]
    fn test_decode_operator_id() {
        let mut msg = [0u8; 25];
        msg[0] = 0x52; // type=5 (OperatorID), version=2
        msg[1] = 0x00; // operator_id_type
        let op_id = b"FIN87astrdge12k8\0\0\0\0";
        msg[2..22].copy_from_slice(op_id);

        match decode_message(&msg) {
            DroneIdMessage::OperatorId(oid) => {
                assert_eq!(oid.operator_id, "FIN87astrdge12k8");
            }
            other => panic!("Expected OperatorId, got {:?}", other),
        }
    }

    #[test]
    fn test_decode_system() {
        let mut msg = [0u8; 25];
        msg[0] = 0x42; // type=4 (System), version=2
        msg[1] = 0x10; // classification_type=1

        // operator lat: 47.0 degrees => 470000000
        let lat: i32 = 470000000;
        msg[2..6].copy_from_slice(&lat.to_le_bytes());

        // operator lon: 8.0 degrees => 80000000
        let lon: i32 = 80000000;
        msg[6..10].copy_from_slice(&lon.to_le_bytes());

        // area_count
        msg[10..12].copy_from_slice(&1u16.to_le_bytes());
        // area_radius: 5 * 10 = 50m
        msg[12] = 5;

        match decode_message(&msg) {
            DroneIdMessage::System(sys) => {
                assert!((sys.operator_latitude - 47.0).abs() < 0.001);
                assert!((sys.operator_longitude - 8.0).abs() < 0.001);
                assert_eq!(sys.area_count, 1);
                assert_eq!(sys.area_radius, 50);
                assert_eq!(sys.classification_type, 1);
            }
            other => panic!("Expected System, got {:?}", other),
        }
    }

    #[test]
    fn test_decode_self_id() {
        let mut msg = [0u8; 25];
        msg[0] = 0x32; // type=3 (SelfID), version=2
        msg[1] = 0x00; // description_type
        let desc = b"Photography flight\0\0\0\0\0";
        msg[2..25].copy_from_slice(desc);

        match decode_message(&msg) {
            DroneIdMessage::SelfId(sid) => {
                assert_eq!(sid.description, "Photography flight");
            }
            other => panic!("Expected SelfId, got {:?}", other),
        }
    }
}
