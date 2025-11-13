//! Session operations abstraction
//!
//! This module provides the session operations pattern that allows
//! transparent switching between plain TCP and TLS connections.
//!
//! The session operations abstraction is the key to supporting both
//! plain and encrypted HTTP connections with the same code.

use super::{Error, Result};
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::os::fd::AsRawFd;
use std::time::Duration;

/// Session operations trait
///
/// This trait defines the operations that can be performed on a session,
/// abstracting over plain TCP and TLS connections.
pub trait SessionOps {
    /// Poll the session for events
    ///
    /// Returns true if the session is ready for the requested operation
    fn poll(&self, events: PollEvents, timeout: Option<Duration>) -> Result<bool>;

    /// Read data from the session
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    /// Write data to the session
    fn write(&mut self, buf: &[u8]) -> Result<usize>;

    /// Close the session
    fn close(&mut self) -> Result<()>;
}

/// Poll events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollEvents {
    Read,
    Write,
    Both,
}

/// HTTP session wrapping a transport with session operations
pub struct HttpSession<S: SessionOps> {
    session: S,
    timeout: Option<Duration>,
}

impl<S: SessionOps> HttpSession<S> {
    /// Create a new HTTP session
    pub fn new(session: S) -> Self {
        HttpSession {
            session,
            timeout: Some(Duration::from_secs(10)),
        }
    }

    /// Set the timeout for operations
    pub fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.timeout = timeout;
    }

    /// Get the timeout
    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    /// Read data with timeout
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        // Poll first
        if !self.session.poll(PollEvents::Read, self.timeout)? {
            return Err(Error::Timeout);
        }

        self.session.read(buf)
    }

    /// Write data with timeout
    pub fn write(&mut self, buf: &[u8]) -> Result<usize> {
        // Poll first
        if !self.session.poll(PollEvents::Write, self.timeout)? {
            return Err(Error::Timeout);
        }

        self.session.write(buf)
    }

    /// Close the session
    pub fn close(&mut self) -> Result<()> {
        self.session.close()
    }

    /// Get a reference to the underlying session
    pub fn get_ref(&self) -> &S {
        &self.session
    }

    /// Get a mutable reference to the underlying session
    pub fn get_mut(&mut self) -> &mut S {
        &mut self.session
    }
}

/// Plain file descriptor session operations
pub struct FdSessionOps {
    stream: TcpStream,
}

impl FdSessionOps {
    /// Create a new FD session operations from a TCP stream
    pub fn new(stream: TcpStream) -> Self {
        FdSessionOps { stream }
    }

    /// Get a reference to the underlying stream
    pub fn stream(&self) -> &TcpStream {
        &self.stream
    }

    /// Get a mutable reference to the underlying stream
    pub fn stream_mut(&mut self) -> &mut TcpStream {
        &mut self.stream
    }
}

impl SessionOps for FdSessionOps {
    fn poll(&self, events: PollEvents, timeout: Option<Duration>) -> Result<bool> {
        use libc::{poll, pollfd, POLLIN, POLLOUT};

        let mut pfd = pollfd {
            fd: self.stream.as_raw_fd(),
            events: match events {
                PollEvents::Read => POLLIN,
                PollEvents::Write => POLLOUT,
                PollEvents::Both => POLLIN | POLLOUT,
            },
            revents: 0,
        };

        let timeout_ms = timeout
            .map(|d| d.as_millis() as i32)
            .unwrap_or(-1); // -1 = infinite

        let result = unsafe { poll(&mut pfd as *mut pollfd, 1, timeout_ms) };

        if result < 0 {
            return Err(Error::Io(io::Error::last_os_error()));
        }

        Ok(result > 0)
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.stream.read(buf).map_err(Error::from)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.stream.write(buf).map_err(Error::from)
    }

    fn close(&mut self) -> Result<()> {
        // Shutdown the connection
        use std::net::Shutdown;
        self.stream
            .shutdown(Shutdown::Both)
            .map_err(Error::from)
    }
}

/// Helper to create an HTTP session from a TCP stream
pub fn from_tcp_stream(stream: TcpStream) -> HttpSession<FdSessionOps> {
    HttpSession::new(FdSessionOps::new(stream))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn test_fd_session_ops() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.write_all(b"Hello").unwrap();
        });

        let stream = TcpStream::connect(addr).unwrap();
        let mut session = FdSessionOps::new(stream);

        // Poll for read
        assert!(session.poll(PollEvents::Read, Some(Duration::from_secs(1))).unwrap());

        // Read data
        let mut buf = [0u8; 5];
        let n = session.read(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buf, b"Hello");

        handle.join().unwrap();
    }

    #[test]
    fn test_http_session_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        // Don't send anything - test timeout
        let _handle = thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
            // Don't send anything
            thread::sleep(Duration::from_secs(2));
        });

        let stream = TcpStream::connect(addr).unwrap();
        let mut session = from_tcp_stream(stream);
        session.set_timeout(Some(Duration::from_millis(100)));

        let mut buf = [0u8; 10];
        let result = session.read(&mut buf);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Timeout));
    }
}
