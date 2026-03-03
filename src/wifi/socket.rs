use std::ffi::CString;
use std::io;
use std::mem;
use std::os::unix::io::RawFd;

const ETH_P_ALL: u16 = 0x0003;

pub struct WifiMonSocket {
    fd: RawFd,
}

impl WifiMonSocket {
    /// Open a raw packet socket bound to the given interface (must be in monitor mode).
    pub fn open(iface: &str) -> io::Result<Self> {
        // Resolve interface index before opening the socket to avoid FD leaks on error
        let ifindex = ifname_to_index(iface)?;

        let fd = unsafe {
            libc::socket(
                libc::AF_PACKET,
                libc::SOCK_RAW | libc::SOCK_CLOEXEC,
                ETH_P_ALL.to_be() as i32,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        let mut addr: libc::sockaddr_ll = unsafe { mem::zeroed() };
        addr.sll_family = libc::AF_PACKET as u16;
        addr.sll_protocol = ETH_P_ALL.to_be();
        addr.sll_ifindex = ifindex as i32;

        let ret = unsafe {
            libc::bind(
                fd,
                &addr as *const libc::sockaddr_ll as *const libc::sockaddr,
                mem::size_of::<libc::sockaddr_ll>() as libc::socklen_t,
            )
        };
        if ret < 0 {
            let err = io::Error::last_os_error();
            unsafe { libc::close(fd) };
            return Err(err);
        }

        Ok(Self { fd })
    }

    /// Read a raw frame into the buffer. Blocking.
    pub fn read_frame(&self, buf: &mut [u8]) -> io::Result<usize> {
        let n = unsafe { libc::read(self.fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(n as usize)
    }
}

impl Drop for WifiMonSocket {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd) };
    }
}

fn ifname_to_index(name: &str) -> io::Result<u32> {
    let cname = CString::new(name)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid interface name"))?;
    let idx = unsafe { libc::if_nametoindex(cname.as_ptr()) };
    if idx == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(idx)
}
