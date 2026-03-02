/// Result of parsing a WiFi beacon carrying a Remote ID payload.
pub struct RemoteIdBeacon {
    pub mac: [u8; 6],
    pub rssi: i8,
    pub counter: u8,
    pub message: [u8; 25],
}

/// OpenDroneID over WiFi Beacon OUI: FA:0B:BC, type 0x0D
const ODID_OUI: [u8; 3] = [0xFA, 0x0B, 0xBC];
const ODID_TYPE: u8 = 0x0D;

/// Vendor Specific IE tag
const IE_VENDOR_SPECIFIC: u8 = 0xDD;

/// Parse a raw captured frame (starting with radiotap header) and extract
/// Remote ID data from WiFi beacons.
///
/// Returns None if the frame is not a beacon or doesn't contain Remote ID data.
pub fn parse_remote_id_beacon(frame: &[u8]) -> Option<RemoteIdBeacon> {
    // Radiotap header: version(1), pad(1), length(2 LE)
    if frame.len() < 4 {
        return None;
    }

    let rt_len = u16::from_le_bytes([frame[2], frame[3]]) as usize;
    if frame.len() < rt_len + 24 {
        // Need at least radiotap + 802.11 header (24 bytes min)
        return None;
    }

    let rssi = extract_rssi(frame, rt_len);

    let dot11 = &frame[rt_len..];

    // Frame Control: first 2 bytes
    let fc = u16::from_le_bytes([dot11[0], dot11[1]]);
    let frame_type = (fc >> 2) & 0x03; // bits 2-3
    let frame_subtype = (fc >> 4) & 0x0F; // bits 4-7

    // We want Management (type=0) Beacon (subtype=8)
    if frame_type != 0 || frame_subtype != 8 {
        return None;
    }

    // Source MAC is addr2: bytes 10-15 of 802.11 header
    if dot11.len() < 24 {
        return None;
    }
    let mut mac = [0u8; 6];
    mac.copy_from_slice(&dot11[10..16]);

    // Beacon frame body starts at byte 24 of 802.11 header
    // Fixed parameters: timestamp(8) + beacon_interval(2) + capability(2) = 12 bytes
    let ie_start = 24 + 12;
    if dot11.len() < ie_start {
        return None;
    }

    // Walk Information Elements
    let ies = &dot11[ie_start..];
    parse_remote_id_ie(ies, mac, rssi)
}

/// Walk IEs and look for Vendor Specific (0xDD) with ODID OUI.
fn parse_remote_id_ie(ies: &[u8], mac: [u8; 6], rssi: i8) -> Option<RemoteIdBeacon> {
    let mut offset = 0;

    while offset + 2 <= ies.len() {
        let tag = ies[offset];
        let len = ies[offset + 1] as usize;
        let data_start = offset + 2;
        let data_end = data_start + len;

        if data_end > ies.len() {
            break;
        }

        // Vendor Specific IE: tag=0xDD, data = OUI(3) + type(1) + payload
        // Remote ID payload: counter(1) + message(25) = 26 bytes
        // Total IE data: 3 (OUI) + 1 (type) + 1 (counter) + 25 (message) = 30
        if tag == IE_VENDOR_SPECIFIC && len >= 30 {
            let ie_data = &ies[data_start..data_end];
            if ie_data[0..3] == ODID_OUI && ie_data[3] == ODID_TYPE {
                let counter = ie_data[4];
                let mut message = [0u8; 25];
                message.copy_from_slice(&ie_data[5..30]);
                return Some(RemoteIdBeacon {
                    mac,
                    rssi,
                    counter,
                    message,
                });
            }
        }

        offset = data_end;
    }

    None
}

/// Extract RSSI (signal dBm) from radiotap header.
/// Walks the radiotap present bitmask to find the signal field.
fn extract_rssi(frame: &[u8], rt_len: usize) -> i8 {
    // Radiotap fields are defined by the "present" bitmask at offset 4.
    // Bit 5 = dB Antenna Signal. We need to walk preceding fields to find offset.
    if rt_len < 8 || frame.len() < rt_len {
        return 0;
    }

    let present = u32::from_le_bytes([frame[4], frame[5], frame[6], frame[7]]);

    // Check if DBM_ANTSIGNAL is present (bit 5)
    if present & (1 << 5) == 0 {
        return 0;
    }

    // Walk fields before bit 5 to compute offset.
    // Field sizes per the radiotap standard (for fields 0-4):
    // 0: TSFT         - 8 bytes (aligned to 8)
    // 1: Flags        - 1 byte
    // 2: Rate         - 1 byte
    // 3: Channel      - 4 bytes (2+2, aligned to 2)
    // 4: FHSS         - 2 bytes
    // 5: dBm Signal   - 1 byte (this is what we want)
    let mut offset = 8usize; // skip radiotap header (version + pad + length + present)

    // Check for extended present bitmasks
    let mut p = present;
    while p & (1 << 31) != 0 {
        offset += 4;
        if offset + 4 > frame.len() {
            return 0;
        }
        p = u32::from_le_bytes([
            frame[offset - 4],
            frame[offset - 3],
            frame[offset - 2],
            frame[offset - 1],
        ]);
    }

    // Bit 0: TSFT (8 bytes, aligned 8)
    if present & (1 << 0) != 0 {
        offset = (offset + 7) & !7; // align to 8
        offset += 8;
    }

    // Bit 1: Flags (1 byte)
    if present & (1 << 1) != 0 {
        offset += 1;
    }

    // Bit 2: Rate (1 byte)
    if present & (1 << 2) != 0 {
        offset += 1;
    }

    // Bit 3: Channel (2 + 2 = 4 bytes, aligned to 2)
    if present & (1 << 3) != 0 {
        offset = (offset + 1) & !1; // align to 2
        offset += 4;
    }

    // Bit 4: FHSS (2 bytes)
    if present & (1 << 4) != 0 {
        offset += 2;
    }

    // Bit 5: dBm Antenna Signal (1 byte) - this is what we want
    if offset < frame.len() {
        frame[offset] as i8
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_remote_id_ie() {
        // Build a vendor-specific IE with ODID payload
        let mut ie = Vec::new();
        // Some non-matching IE first
        ie.push(0x00); // SSID tag
        ie.push(0x04); // length
        ie.extend_from_slice(b"test");

        // ODID Vendor Specific IE
        ie.push(IE_VENDOR_SPECIFIC);
        ie.push(30); // length: OUI(3) + type(1) + counter(1) + message(25)
        ie.extend_from_slice(&ODID_OUI);
        ie.push(ODID_TYPE);
        ie.push(0x42); // counter

        // 25-byte message: BasicId
        let mut msg = [0u8; 25];
        msg[0] = 0x02; // type=0 (BasicID), version=2
        msg[1] = 0x12; // id_type=serial, ua_type=helicopter
        ie.extend_from_slice(&msg);

        let mac = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        let result = parse_remote_id_ie(&ie, mac, -50);
        assert!(result.is_some());
        let beacon = result.unwrap();
        assert_eq!(beacon.mac, mac);
        assert_eq!(beacon.rssi, -50);
        assert_eq!(beacon.counter, 0x42);
        assert_eq!(beacon.message[0], 0x02);
    }

    #[test]
    fn test_no_odid_ie() {
        let ie = vec![0x00, 0x04, b't', b'e', b's', b't'];
        let mac = [0; 6];
        assert!(parse_remote_id_ie(&ie, mac, 0).is_none());
    }
}
