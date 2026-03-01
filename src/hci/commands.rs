use crate::hci::socket::HciSocket;
use std::io;

const OGF_LE_CTL: u16 = 0x08;
const OCF_LE_SET_SCAN_PARAMS: u16 = 0x000B;
const OCF_LE_SET_SCAN_ENABLE: u16 = 0x000C;

/// Configure LE scan parameters: passive scan, 100ms interval/window.
pub fn le_set_scan_parameters(sock: &HciSocket) -> io::Result<()> {
    let params: [u8; 7] = [
        0x00, // scan type: passive
        0xA0, 0x00, // scan interval: 0x00A0 = 160 * 0.625ms = 100ms
        0xA0, 0x00, // scan window: 0x00A0 = 100ms
        0x00, // own address type: public
        0x00, // filter policy: accept all
    ];
    sock.send_command(OGF_LE_CTL, OCF_LE_SET_SCAN_PARAMS, &params)
}

/// Enable or disable LE scanning.
pub fn le_set_scan_enable(sock: &HciSocket, enable: bool) -> io::Result<()> {
    let params: [u8; 2] = [
        if enable { 0x01 } else { 0x00 }, // enable
        0x00,                             // filter duplicates: disabled
    ];
    sock.send_command(OGF_LE_CTL, OCF_LE_SET_SCAN_ENABLE, &params)
}
