//! The only `unsafe` code in the server: reading `TCP_INFO` for a connected socket.

use std::os::fd::RawFd;

/// The kernel's round-trip-time estimate for one TCP connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TcpRtt {
    /// Smoothed RTT (`tcpi_rtt`), microseconds.
    pub rtt_us: u32,
    /// RTT variance (`tcpi_rttvar`), microseconds.
    pub rttvar_us: u32,
}

/// `getsockopt(fd, IPPROTO_TCP, TCP_INFO)` → smoothed RTT and its variance.
///
/// Returns `None` if the call fails, the kernel returned a truncated struct, or no RTT sample
/// exists yet (`tcpi_rtt == 0`).
///
/// The caller must pass the descriptor of a socket it currently owns: the connection layer calls
/// this from inside hyper's `Service::call`, which hyper only invokes while it polls the
/// connection that owns the socket, so `fd` cannot have been closed or reused at that point.
#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
pub(crate) fn tcp_rtt(fd: RawFd) -> Option<TcpRtt> {
    let mut info = std::mem::MaybeUninit::<libc::tcp_info>::zeroed();
    let mut len = std::mem::size_of::<libc::tcp_info>() as libc::socklen_t;
    // SAFETY: `info` points to a zero-initialised `tcp_info` (plain old data, so all-zero bytes
    // are a valid value) of exactly `len` bytes; the kernel writes at most `len` bytes and stores
    // the number written in `len`. `fd` is only read by the kernel: an invalid descriptor makes
    // the call fail with EBADF/ENOTSOCK, it cannot cause undefined behaviour.
    let (rc, info) = unsafe {
        let rc = libc::getsockopt(
            fd,
            libc::IPPROTO_TCP,
            libc::TCP_INFO,
            info.as_mut_ptr().cast::<libc::c_void>(),
            &mut len,
        );
        (rc, info.assume_init())
    };
    let needed = std::mem::offset_of!(libc::tcp_info, tcpi_rttvar) + std::mem::size_of::<u32>();
    if rc != 0 || (len as usize) < needed || info.tcpi_rtt == 0 {
        return None;
    }
    Some(TcpRtt { rtt_us: info.tcpi_rtt, rttvar_us: info.tcpi_rttvar })
}

/// `TCP_INFO` is Linux-specific; elsewhere the RTT is reported as unavailable.
#[cfg(not(target_os = "linux"))]
pub(crate) fn tcp_rtt(_fd: RawFd) -> Option<TcpRtt> {
    None
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::os::fd::AsRawFd;

    #[test]
    fn reads_rtt_of_a_loopback_connection() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let client = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        // The SYN-ACK → ACK exchange gives the server side its first RTT sample.
        let rtt = tcp_rtt(server.as_raw_fd()).expect("TCP_INFO on a connected socket");
        assert!(rtt.rtt_us > 0);
        drop(client);
    }

    #[test]
    fn invalid_descriptor_is_none() {
        assert_eq!(tcp_rtt(-1), None);
        // A non-socket descriptor (stdin may be a pipe or tty) is rejected by the kernel too.
        let file = std::fs::File::open("/proc/self/stat").unwrap();
        assert_eq!(tcp_rtt(file.as_raw_fd()), None);
    }
}
