//! TLS session operations
//!
//! This module implements the SessionOps trait for TLS connections,
//! enabling transparent switching between plain TCP and TLS I/O.

use super::config::{TlsConfig, TlsError};
use super::vars::TlsVars;
use crate::http::session::{SessionOps, PollEvents};
use crate::http::{Error, Result as HttpResult};
use openssl::ssl::{Ssl, SslStream};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::fd::AsRawFd;
use std::time::Duration;

/// TLS session operations
///
/// Implements SessionOps trait for TLS-encrypted connections.
/// Wraps an OpenSSL SslStream and provides poll/read/write/close operations.
pub struct TlsSessionOps {
    stream: SslStream<TcpStream>,
    _config: TlsConfig,
    vars: TlsVars,
    failed: bool,
}

impl TlsSessionOps {
    /// Create a client TLS connection (perform handshake)
    pub fn connect(tcp_stream: TcpStream, config: TlsConfig) -> std::result::Result<Self, TlsError> {
        // Create SSL connection
        let mut ssl = Ssl::new(&config.ctx)?;

        // Set SNI servername if configured
        if let Some(ref servername) = config.servername {
            ssl.set_hostname(servername)?;
        }

        // Request OCSP staple if configured
        if config.cert_status {
            // Enable status request
            ssl.set_status_type(openssl::ssl::StatusType::OCSP)?;
        }

        // Keep in blocking mode for handshake
        // The openssl crate's connect() method handles the handshake synchronously
        let ssl_stream = match ssl.connect(tcp_stream) {
            Ok(stream) => stream,
            Err(e) => {
                return Err(TlsError::HandshakeFailed(format!("Connection failed: {}", e)));
            }
        };

        // Create TLS vars from the SSL session
        let vars = TlsVars::from_ssl(ssl_stream.ssl(), false);

        Ok(TlsSessionOps {
            stream: ssl_stream,
            _config: config,
            vars,
            failed: false,
        })
    }

    /// Accept a client connection with TLS (perform handshake)
    pub fn accept(tcp_stream: TcpStream, config: TlsConfig) -> std::result::Result<Self, TlsError> {
        // Create SSL connection
        let ssl = Ssl::new(&config.ctx)?;

        // Keep in blocking mode for handshake
        // The openssl crate's accept() method handles the handshake synchronously
        let ssl_stream = match ssl.accept(tcp_stream) {
            Ok(stream) => stream,
            Err(e) => {
                return Err(TlsError::HandshakeFailed(format!("Accept failed: {}", e)));
            }
        };

        // Create TLS vars from the SSL session
        let vars = TlsVars::from_ssl(ssl_stream.ssl(), false);

        Ok(TlsSessionOps {
            stream: ssl_stream,
            _config: config,
            vars,
            failed: false,
        })
    }

    /// Get TLS variables (for expect commands)
    pub fn vars(&self) -> &TlsVars {
        &self.vars
    }

    /// Get mutable TLS variables
    pub fn vars_mut(&mut self) -> &mut TlsVars {
        &mut self.vars
    }

    /// Check if TLS failed
    pub fn failed(&self) -> bool {
        self.failed
    }

    /// Get reference to underlying TCP stream
    pub fn get_ref(&self) -> &TcpStream {
        self.stream.get_ref()
    }

    /// Get mutable reference to underlying TCP stream
    pub fn get_mut(&mut self) -> &mut TcpStream {
        self.stream.get_mut()
    }
}

impl SessionOps for TlsSessionOps {
    fn poll(&self, events: PollEvents, timeout: Option<Duration>) -> HttpResult<bool> {
        use libc::{poll, pollfd, POLLIN, POLLOUT};

        // Check if SSL has pending data
        if events == PollEvents::Read || events == PollEvents::Both {
            if self.stream.ssl().pending() > 0 {
                return Ok(true);
            }
        }

        // Poll the underlying file descriptor
        let mut pfd = pollfd {
            fd: self.stream.get_ref().as_raw_fd(),
            events: match events {
                PollEvents::Read => POLLIN,
                PollEvents::Write => POLLOUT,
                PollEvents::Both => POLLIN | POLLOUT,
            },
            revents: 0,
        };

        let timeout_ms = timeout
            .map(|d| d.as_millis() as i32)
            .unwrap_or(-1);

        let result = unsafe { poll(&mut pfd as *mut pollfd, 1, timeout_ms) };

        if result < 0 {
            return Err(Error::Io(std::io::Error::last_os_error()));
        }

        Ok(result > 0)
    }

    fn read(&mut self, buf: &mut [u8]) -> HttpResult<usize> {
        match self.stream.read(buf) {
            Ok(n) => Ok(n),
            Err(e) => {
                self.failed = true;
                self.vars.failed = true;
                Err(Error::Io(e))
            }
        }
    }

    fn write(&mut self, buf: &[u8]) -> HttpResult<usize> {
        match self.stream.write(buf) {
            Ok(n) => Ok(n),
            Err(e) => {
                self.failed = true;
                self.vars.failed = true;
                Err(Error::Io(e))
            }
        }
    }

    fn flush(&mut self) -> HttpResult<()> {
        self.stream.flush().map_err(|e| {
            self.failed = true;
            self.vars.failed = true;
            Error::Io(e)
        })
    }

    fn close(&mut self) -> HttpResult<()> {
        // Perform SSL shutdown if not failed
        if !self.failed {
            let _ = self.stream.shutdown();
        }

        // Shutdown the underlying TCP connection
        use std::net::Shutdown;
        self.stream.get_mut()
            .shutdown(Shutdown::Both)
            .map_err(Error::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::TlsConfig;
    use super::super::TlsVersion;
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_tls_client_server_handshake() {
        // Create server
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        // Server configuration
        let server_config = TlsConfig::server()
            .version(TlsVersion::Tls13)
            .build()
            .unwrap();

        // Client configuration
        let client_config = TlsConfig::client()
            .version(TlsVersion::Tls13)
            .verify_peer(false)
            .build()
            .unwrap();

        // Spawn server thread
        let server_handle = thread::spawn(move || {
            let (tcp_stream, _) = listener.accept().unwrap();
            let mut tls_session = server_config.accept(tcp_stream).unwrap();

            assert!(!tls_session.failed());

            // Read from client
            let mut buf = vec![0u8; 5];
            let n = tls_session.read(&mut buf).unwrap();
            assert_eq!(n, 5);
            assert_eq!(&buf[..n], b"Hello");

            // Write to client
            let n = tls_session.write(b"World").unwrap();
            assert_eq!(n, 5);

            tls_session.close().unwrap();
        });

        // Give server time to start
        thread::sleep(Duration::from_millis(100));

        // Connect client
        let tcp_stream = TcpStream::connect(addr).unwrap();
        let mut tls_session = client_config.connect(tcp_stream).unwrap();

        assert!(!tls_session.failed());

        // Write to server
        let n = tls_session.write(b"Hello").unwrap();
        assert_eq!(n, 5);

        // Read from server
        let mut buf = vec![0u8; 5];
        let n = tls_session.read(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buf[..n], b"World");

        tls_session.close().unwrap();

        server_handle.join().unwrap();
    }

    #[test]
    fn test_tls_vars_after_handshake() {
        // Create server
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server_config = TlsConfig::server()
            .version(TlsVersion::Tls13)
            .build()
            .unwrap();

        let client_config = TlsConfig::client()
            .version(TlsVersion::Tls13)
            .verify_peer(false)
            .build()
            .unwrap();

        let server_handle = thread::spawn(move || {
            let (tcp_stream, _) = listener.accept().unwrap();
            let tls_session = server_config.accept(tcp_stream).unwrap();

            // Check TLS vars
            let vars = tls_session.vars();
            assert!(!vars.failed);
            assert!(vars.version.contains("TLS"));
        });

        thread::sleep(Duration::from_millis(100));

        let tcp_stream = TcpStream::connect(addr).unwrap();
        let tls_session = client_config.connect(tcp_stream).unwrap();

        // Check TLS vars
        let vars = tls_session.vars();
        assert!(!vars.failed);
        assert!(vars.version.contains("TLS"));

        server_handle.join().unwrap();
    }
}
