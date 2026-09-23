//! CANDIDATE SOCK_SEQPACKET transport implementing the narrow
//! [`DecisionTransport`] interface.  NOT ACCEPTED: the concrete byte transport
//! and its deadline handling are the subject of the independent review's open
//! transport-repair items (findings 1/4/6).  The barrier's acceptance logic does
//! not depend on this file.  It exists to (a) give the seam a real inherited
//! endpoint and (b) let the descriptor-inheritance subprocess tests drive a real
//! kernel socketpair.  Second-resolution wall time bounds sub-second deadline
//! precision here; the barrier rechecks the deadline at the irreversible
//! acceptance transition, which is the security-critical guarantee.

use crate::decision::{DecisionAck, DecisionTransport, RawDecision, TransportError};
use crate::wire;
use std::io;
use std::os::unix::io::RawFd;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct SocketTransport {
    fd: RawFd,
    valid: bool,
    closed: bool,
}

impl SocketTransport {
    pub fn new(fd: RawFd) -> Self {
        let valid = validate_socket_and_nonblocking(fd);
        Self {
            fd,
            valid,
            closed: false,
        }
    }
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(u64::MAX)
}

/// Fail-closed unless the descriptor is a real socket, and force nonblocking so
/// neither poll nor recv can block past the bounded deadline.
fn validate_socket_and_nonblocking(fd: RawFd) -> bool {
    if fd < 0 {
        return false;
    }
    // SAFETY: fd is a process-owned descriptor; fstat/fcntl take valid pointers.
    unsafe {
        let mut stat: libc::stat = std::mem::zeroed();
        if libc::fstat(fd, &mut stat) != 0 {
            return false;
        }
        if (stat.st_mode & libc::S_IFMT) != libc::S_IFSOCK {
            return false;
        }
        let flags = libc::fcntl(fd, libc::F_GETFL);
        if flags < 0 {
            return false;
        }
        if libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) != 0 {
            return false;
        }
    }
    true
}

impl DecisionTransport for SocketTransport {
    fn receive(&mut self, deadline_epoch: u64) -> Result<Option<RawDecision>, TransportError> {
        if !self.valid || self.closed {
            return Err(TransportError::Io(libc::EBADF));
        }
        loop {
            let now = now_epoch();
            if now >= deadline_epoch {
                return Err(TransportError::Expired);
            }
            let remaining = deadline_epoch - now;
            let timeout_ms = remaining
                .saturating_mul(1000)
                .saturating_add(1)
                .min(i32::MAX as u64) as i32;
            match poll_readable(self.fd, timeout_ms) {
                Ok(false) => return Err(TransportError::Expired),
                Err(err) => return Err(err),
                Ok(true) => {}
            }
            match recvmsg_one(self.fd) {
                Ok(Recv::Message(bytes, truncated)) => {
                    return match wire::decode_decision(&bytes, truncated) {
                        Ok(decoded) => Ok(Some(RawDecision {
                            action: decoded.action.to_string(),
                            nonce_hex: decoded.nonce_hex,
                            clock: decoded.clock,
                            state_hex: decoded.state_hex,
                            token_hex: decoded.token_hex,
                            source: decoded.source,
                            profile: decoded.profile,
                            rest: decoded.rest,
                            attempt: decoded.attempt,
                            deadline_epoch: decoded.deadline_epoch,
                        })),
                        Err(_) => Err(TransportError::Malformed),
                    };
                }
                // Genuine orderly shutdown: the peer closed without a decision.
                Ok(Recv::Eof) => return Err(TransportError::Eof),
                // Spurious nonblocking miss (EINTR/EAGAIN): recheck the deadline
                // and poll again, never treat it as EOF or use a stale timeout.
                Ok(Recv::WouldBlock) => continue,
                Err(err) => return Err(err),
            }
        }
    }

    fn acknowledge(&mut self, ack: &DecisionAck) {
        if !self.valid || self.closed {
            return;
        }
        let Ok(bytes) = wire::encode_ack(ack) else {
            return;
        };
        // Best effort, bounded, nonblocking: the barrier has already latched its
        // irreversible decision; a lost ack is the supervisor's UNKNOWN.
        // SAFETY: `bytes` is a valid readable slice for the call's duration.
        unsafe {
            libc::send(
                self.fd,
                bytes.as_ptr() as *const libc::c_void,
                bytes.len(),
                libc::MSG_NOSIGNAL | libc::MSG_DONTWAIT,
            );
        }
    }

    fn close(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        // SAFETY: fd is a process-owned socket; shutdown/close accept it by value.
        unsafe {
            libc::shutdown(self.fd, libc::SHUT_RDWR);
            libc::close(self.fd);
        }
    }
}

fn poll_readable(fd: RawFd, timeout_ms: i32) -> Result<bool, TransportError> {
    let mut pfd = libc::pollfd {
        fd,
        events: libc::POLLIN | libc::POLLHUP,
        revents: 0,
    };
    loop {
        // SAFETY: a single valid pollfd with matching count.
        let rc = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };
        if rc < 0 {
            let err = io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            return Err(TransportError::Io(err.raw_os_error().unwrap_or(libc::EIO)));
        }
        if rc == 0 {
            return Ok(false);
        }
        return Ok((pfd.revents & (libc::POLLIN | libc::POLLHUP)) != 0);
    }
}

enum Recv {
    Message(Vec<u8>, bool),
    /// Peer performed an orderly shutdown with no pending decision (rc == 0).
    Eof,
    /// EINTR/EAGAIN: no message now, not an EOF — recheck deadline and poll.
    WouldBlock,
}

fn recvmsg_one(fd: RawFd) -> Result<Recv, TransportError> {
    let mut buf = vec![0_u8; wire::MAX_FRAME];
    let mut iov = libc::iovec {
        iov_base: buf.as_mut_ptr() as *mut libc::c_void,
        iov_len: buf.len(),
    };
    let mut header = libc::msghdr {
        msg_name: std::ptr::null_mut(),
        msg_namelen: 0,
        msg_iov: &mut iov,
        msg_iovlen: 1,
        msg_control: std::ptr::null_mut(),
        msg_controllen: 0,
        msg_flags: 0,
    };
    // SAFETY: buf/iov/header are valid and live for the call's duration.
    let rc = unsafe { libc::recvmsg(fd, &mut header, libc::MSG_TRUNC | libc::MSG_DONTWAIT) };
    if rc < 0 {
        let err = io::Error::last_os_error();
        let code = err.raw_os_error().unwrap_or(libc::EIO);
        if code == libc::EINTR || code == libc::EAGAIN || code == libc::EWOULDBLOCK {
            return Ok(Recv::WouldBlock);
        }
        return Err(TransportError::Io(code));
    }
    if rc == 0 {
        return Ok(Recv::Eof);
    }
    let truncated = (header.msg_flags & libc::MSG_TRUNC) != 0;
    buf.truncate(rc as usize);
    Ok(Recv::Message(buf, truncated))
}
