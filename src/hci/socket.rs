use std::io;
use std::mem;
use std::os::unix::io::{AsRawFd, RawFd};

const AF_BLUETOOTH: i32 = 31;
const BTPROTO_HCI: i32 = 1;
const SOL_HCI: i32 = 0;
const HCI_FILTER: i32 = 2;

// HCI packet types
const HCI_EVENT_PKT: u8 = 0x04;

// LE Meta Event code
const LE_META_EVENT: u8 = 0x3E;

#[repr(C)]
#[derive(Default)]
struct SockaddrHci {
    hci_family: u16,
    hci_dev: u16,
    hci_channel: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct HciFilter {
    type_mask: u32,
    event_mask: [u32; 2],
    opcode: u16,
}

impl HciFilter {
    fn set_ptype(&mut self, ptype: u8) {
        self.type_mask |= 1u32 << (ptype as u32);
    }

    fn set_event(&mut self, event: u8) {
        let idx = (event >> 5) as usize;
        let bit = event & 0x1F;
        self.event_mask[idx] |= 1u32 << (bit as u32);
    }
}

pub struct HciSocket {
    fd: RawFd,
}

impl HciSocket {
    /// Open a raw HCI socket bound to the given adapter index.
    pub fn open(dev_id: u16) -> io::Result<Self> {
        let fd = unsafe {
            libc::socket(
                AF_BLUETOOTH,
                libc::SOCK_RAW | libc::SOCK_CLOEXEC,
                BTPROTO_HCI,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        let addr = SockaddrHci {
            hci_family: AF_BLUETOOTH as u16,
            hci_dev: dev_id,
            hci_channel: 0, // HCI_CHANNEL_RAW
        };

        let ret = unsafe {
            libc::bind(
                fd,
                &addr as *const SockaddrHci as *const libc::sockaddr,
                mem::size_of::<SockaddrHci>() as libc::socklen_t,
            )
        };
        if ret < 0 {
            let err = io::Error::last_os_error();
            unsafe {
                libc::close(fd);
            }
            return Err(err);
        }

        // Set HCI filter to only pass HCI Event packets with LE Meta Event
        let mut filter = HciFilter::default();
        filter.set_ptype(HCI_EVENT_PKT);
        filter.set_event(LE_META_EVENT);

        let ret = unsafe {
            libc::setsockopt(
                fd,
                SOL_HCI,
                HCI_FILTER,
                &filter as *const HciFilter as *const libc::c_void,
                mem::size_of::<HciFilter>() as libc::socklen_t,
            )
        };
        if ret < 0 {
            let err = io::Error::last_os_error();
            unsafe {
                libc::close(fd);
            }
            return Err(err);
        }

        Ok(Self { fd })
    }

    /// Send a raw HCI command.
    pub fn send_command(&self, ogf: u16, ocf: u16, params: &[u8]) -> io::Result<()> {
        let opcode = ((ogf & 0x3F) << 10) | (ocf & 0x3FF);
        let plen = params.len() as u8;

        // HCI command packet: type(1) + opcode(2) + plen(1) + params
        let mut buf = Vec::with_capacity(4 + params.len());
        buf.push(0x01); // HCI Command packet type
        buf.push((opcode & 0xFF) as u8);
        buf.push((opcode >> 8) as u8);
        buf.push(plen);
        buf.extend_from_slice(params);

        let written =
            unsafe { libc::write(self.fd, buf.as_ptr() as *const libc::c_void, buf.len()) };
        if written < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Read a raw HCI event into the provided buffer. Returns the number of bytes read.
    pub fn read_event(&self, buf: &mut [u8]) -> io::Result<usize> {
        let n = unsafe { libc::read(self.fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(n as usize)
    }
}

impl AsRawFd for HciSocket {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl Drop for HciSocket {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}
