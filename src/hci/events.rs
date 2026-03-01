/// A parsed LE Advertising Report.
#[derive(Debug, Clone)]
pub struct AdvertisingReport {
    pub event_type: u8,
    pub addr_type: u8,
    pub addr: [u8; 6],
    pub data: Vec<u8>,
    pub rssi: i8,
}

/// Parse an HCI event buffer (after the 0x04 packet indicator byte).
/// Returns advertising reports if this is a LE Advertising Report event.
pub fn parse_hci_event(buf: &[u8]) -> Option<Vec<AdvertisingReport>> {
    // HCI event: event_code(1) + param_len(1) + params...
    if buf.len() < 2 {
        return None;
    }
    let event_code = buf[0];
    let param_len = buf[1] as usize;

    if event_code != 0x3E {
        // Not a LE Meta Event
        return None;
    }

    if buf.len() < 2 + param_len || param_len < 1 {
        return None;
    }

    let params = &buf[2..2 + param_len];
    let subevent = params[0];

    if subevent != 0x02 {
        // Not LE Advertising Report
        return None;
    }

    if params.len() < 2 {
        return None;
    }

    let num_reports = params[1] as usize;
    let mut offset = 2;
    let mut reports = Vec::with_capacity(num_reports);

    for _ in 0..num_reports {
        // event_type(1) + addr_type(1) + addr(6) + data_length(1)
        if offset + 9 > params.len() {
            break;
        }

        let event_type = params[offset];
        let addr_type = params[offset + 1];
        let mut addr = [0u8; 6];
        addr.copy_from_slice(&params[offset + 2..offset + 8]);
        let data_length = params[offset + 8] as usize;
        offset += 9;

        if offset + data_length > params.len() {
            break;
        }

        let data = params[offset..offset + data_length].to_vec();
        offset += data_length;

        // RSSI is 1 byte after the data
        if offset >= params.len() {
            break;
        }
        let rssi = params[offset] as i8;
        offset += 1;

        reports.push(AdvertisingReport {
            event_type,
            addr_type,
            addr,
            data,
            rssi,
        });
    }

    Some(reports)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_advertising_report() {
        // LE Meta Event with 1 advertising report
        let mut buf = vec![
            0x3E, // event code: LE Meta Event
            0x00, // param_len (will fill)
            0x02, // subevent: LE Advertising Report
            0x01, // num_reports: 1
            0x00, // event_type: ADV_IND
            0x01, // addr_type: random
            0xAA,
            0xBB,
            0xCC,
            0xDD,
            0xEE,
            0xFF, // addr
            0x03, // data_length
            0x02,
            0x01,
            0x06,         // data (flags AD structure)
            0xC0u8 as u8, // RSSI: -64
        ];
        buf[1] = (buf.len() - 2) as u8;

        let reports = parse_hci_event(&buf).unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].addr, [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        assert_eq!(reports[0].data, vec![0x02, 0x01, 0x06]);
        assert_eq!(reports[0].rssi, -64);
    }
}
