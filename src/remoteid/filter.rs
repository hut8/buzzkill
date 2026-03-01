/// The AD Type for Service Data - 16-bit UUID.
const AD_TYPE_SERVICE_DATA_16: u8 = 0x16;
/// The Open Drone ID UUID (little-endian: 0xFA, 0xFF).
const ODID_UUID_LO: u8 = 0xFA;
const ODID_UUID_HI: u8 = 0xFF;
/// The application code for ASTM Remote ID over BLE.
const ODID_APP_CODE: u8 = 0x0D;

/// Result of filtering an advertising data payload.
/// Contains the counter byte and the 25-byte OpenDroneID message.
#[derive(Debug, Clone)]
pub struct RemoteIdPayload {
    pub counter: u8,
    pub message: [u8; 25],
}

/// Walk AD structures in BLE advertising data and extract OpenDroneID payloads.
/// Returns all matching payloads found (usually 0 or 1).
pub fn extract_remote_id(ad_data: &[u8]) -> Vec<RemoteIdPayload> {
    let mut results = Vec::new();
    let mut offset = 0;

    while offset < ad_data.len() {
        if offset + 1 > ad_data.len() {
            break;
        }
        let length = ad_data[offset] as usize;
        if length == 0 {
            break;
        }
        let struct_start = offset + 1;
        let struct_end = struct_start + length;
        if struct_end > ad_data.len() {
            break;
        }

        // AD structure: length(1) + ad_type(1) + data(length-1)
        // For Remote ID: ad_type=0x16, then UUID_lo(1) + UUID_hi(1) + app_code(1) + counter(1) + message(25)
        // Minimum length for Remote ID AD: 1(type) + 2(UUID) + 1(app_code) + 1(counter) + 25(message) = 30
        if length >= 30 {
            let ad_type = ad_data[struct_start];
            if ad_type == AD_TYPE_SERVICE_DATA_16
                && ad_data[struct_start + 1] == ODID_UUID_LO
                && ad_data[struct_start + 2] == ODID_UUID_HI
                && ad_data[struct_start + 3] == ODID_APP_CODE
            {
                let counter = ad_data[struct_start + 4];
                let msg_start = struct_start + 5;
                if msg_start + 25 <= struct_end {
                    let mut message = [0u8; 25];
                    message.copy_from_slice(&ad_data[msg_start..msg_start + 25]);
                    results.push(RemoteIdPayload { counter, message });
                }
            }
        }

        offset = struct_end;
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_remote_id_basic() {
        // Build a fake AD structure with Remote ID payload
        let mut ad_data = Vec::new();

        // A non-matching AD structure first (flags)
        ad_data.push(0x02); // length
        ad_data.push(0x01); // AD Type: Flags
        ad_data.push(0x06); // flags value

        // Remote ID AD structure
        // length = 1(type) + 2(UUID) + 1(app_code) + 1(counter) + 25(message) = 30
        ad_data.push(30);
        ad_data.push(AD_TYPE_SERVICE_DATA_16);
        ad_data.push(ODID_UUID_LO);
        ad_data.push(ODID_UUID_HI);
        ad_data.push(ODID_APP_CODE);
        ad_data.push(0x42); // counter

        // 25-byte message: Basic ID, protocol version 2, ID type=serial, UA type=helicopter
        let mut msg = [0u8; 25];
        msg[0] = 0x02; // type=0 (Basic ID), proto version=2
        msg[1] = 0x10; // ID type=1 (serial number) << 4 | UA type=0 (none)
                       // serial number in bytes 2..22
        let serial = b"SERIAL123456789A\0\0\0\0";
        msg[2..22].copy_from_slice(serial);
        ad_data.extend_from_slice(&msg);

        let payloads = extract_remote_id(&ad_data);
        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].counter, 0x42);
        assert_eq!(payloads[0].message[0], 0x02);
    }

    #[test]
    fn test_no_remote_id() {
        let ad_data = vec![0x02, 0x01, 0x06]; // just flags
        let payloads = extract_remote_id(&ad_data);
        assert!(payloads.is_empty());
    }
}
