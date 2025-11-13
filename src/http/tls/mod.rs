//! TLS support for HTTP connections
//!
//! This module implements TLS (Transport Layer Security) for HTTP clients and servers,
//! enabling encrypted HTTPS connections for testing. It's a port of the C vtc_tls.c
//! implementation to Rust.
//!
//! # Architecture
//!
//! The TLS implementation uses the session operations abstraction pattern:
//!
//! 1. `TlsConfig` defines TLS settings (versions, ciphers, certificates)
//! 2. `TlsSessionOps` implements the `SessionOps` trait for encrypted I/O
//! 3. All HTTP code remains unchanged - it transparently uses TLS operations
//!
//! # Features
//!
//! - TLS 1.0 through TLS 1.3 support (OpenSSL version dependent)
//! - Certificate loading and validation
//! - ALPN (Application-Layer Protocol Negotiation)
//! - SNI (Server Name Indication)
//! - Session resumption
//! - OCSP stapling
//! - Client certificate verification
//!
//! # Examples
//!
//! ## Client with TLS
//!
//! ```no_run
//! use vtest2::http::tls::{TlsConfig, TlsVersion};
//! use vtest2::http::HttpClient;
//! use std::net::TcpStream;
//!
//! let tls_config = TlsConfig::client()
//!     .version(TlsVersion::Tls13)
//!     .servername("example.com")
//!     .verify_peer(true)
//!     .build()
//!     .unwrap();
//!
//! let tcp_stream = TcpStream::connect("example.com:443").unwrap();
//! let tls_session = tls_config.connect(tcp_stream).unwrap();
//! let mut client = HttpClient::from_tls(tls_session);
//! ```
//!
//! ## Server with TLS
//!
//! ```no_run
//! use vtest2::http::tls::{TlsConfig, TlsVersion, ClientVerify};
//! use vtest2::http::HttpServer;
//! use std::net::TcpListener;
//!
//! let tls_config = TlsConfig::server()
//!     .cert_file("server.pem")
//!     .unwrap()
//!     .version_range(TlsVersion::Tls12, TlsVersion::Tls13)
//!     .client_verify(ClientVerify::Optional)
//!     .build()
//!     .unwrap();
//!
//! let listener = TcpListener::bind("127.0.0.1:443").unwrap();
//! let (tcp_stream, _) = listener.accept().unwrap();
//! let tls_session = tls_config.accept(tcp_stream).unwrap();
//! let mut server = HttpServer::from_tls(tls_session);
//! ```

pub mod config;
pub mod session;
pub mod handshake;
pub mod cert;
pub mod vars;
pub mod builtin_cert;

pub use config::{
    TlsConfig, TlsConfigBuilder, TlsVersion, ClientVerify, TlsError,
    ClientConfigBuilder, ServerConfigBuilder,
};
pub use session::TlsSessionOps;
pub use vars::TlsVars;
pub use cert::CertInfo;

/// Result type for TLS operations
pub type Result<T> = std::result::Result<T, TlsError>;
