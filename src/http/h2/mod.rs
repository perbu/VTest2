//! HTTP/2 protocol implementation
//!
//! This module provides HTTP/2 client and server functionality for testing,
//! ported from the C vtc_http2.c implementation to Rust.
//!
//! # Architecture
//!
//! The HTTP/2 implementation uses the `h2` crate for core protocol handling
//! (frame parsing, HPACK compression, stream multiplexing, flow control) and
//! provides a testing-friendly wrapper API that integrates with VTest2's
//! architecture.
//!
//! ## Features
//!
//! - **Frame handling**: All HTTP/2 frame types (DATA, HEADERS, PRIORITY,
//!   RST_STREAM, SETTINGS, PUSH_PROMISE, PING, GOAWAY, WINDOW_UPDATE, CONTINUATION)
//! - **Stream multiplexing**: Multiple concurrent streams per connection
//! - **HPACK compression**: Header compression/decompression (via h2 crate)
//! - **Flow control**: Connection and stream-level window management
//! - **ALPN integration**: Protocol negotiation via TLS
//! - **Settings exchange**: Initial connection setup and configuration
//! - **Priority handling**: Stream priority and dependencies
//! - **Server push**: PUSH_PROMISE frames
//!
//! # Examples
//!
//! ## HTTP/2 Client
//!
//! ```no_run
//! use vtest2::http::h2::{H2Client, H2ClientBuilder};
//! use vtest2::http::tls::{TlsConfig, TlsVersion};
//! use std::net::TcpStream;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create TLS config with ALPN for HTTP/2
//! let tls_config = TlsConfig::client()
//!     .version(TlsVersion::Tls13)
//!     .servername("example.com")
//!     .alpn(&["h2"])
//!     .build()?;
//!
//! // Connect with TLS
//! let tcp_stream = TcpStream::connect("example.com:443")?;
//! let tls_session = tls_config.connect(tcp_stream)?;
//!
//! // Create HTTP/2 client
//! let mut client = H2ClientBuilder::new()
//!     .build(tls_session)
//!     .await?;
//!
//! // Send request
//! let response = client.get("/").await?;
//! println!("Status: {}", response.status());
//! # Ok(())
//! # }
//! ```
//!
//! ## HTTP/2 Server
//!
//! ```no_run
//! use vtest2::http::h2::{H2Server, H2ServerBuilder};
//! use vtest2::http::tls::{TlsConfig, TlsVersion};
//! use std::net::TcpListener;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create TLS config with ALPN for HTTP/2
//! let tls_config = TlsConfig::server()
//!     .cert_file("server.pem")?
//!     .version(TlsVersion::Tls13)
//!     .alpn(&["h2"])
//!     .build()?;
//!
//! // Accept connection
//! let listener = TcpListener::bind("127.0.0.1:443")?;
//! let (tcp_stream, _) = listener.accept()?;
//! let tls_session = tls_config.accept(tcp_stream)?;
//!
//! // Create HTTP/2 server
//! let mut server = H2ServerBuilder::new()
//!     .build(tls_session)
//!     .await?;
//!
//! // Process request
//! let request = server.receive_request().await?;
//! server.send_response(200, &[], b"OK").await?;
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod server;
pub mod stream;
pub mod frames;
pub mod flow_control;
pub mod settings;
pub mod error;
pub mod codec;

pub use client::{H2Client, H2ClientBuilder};
pub use server::{H2Server, H2ServerBuilder};
pub use stream::{StreamId, StreamState, H2Stream};
pub use frames::{Frame, FrameType, FrameFlags, DataFrame, HeadersFrame, SettingsFrame};
pub use settings::{Settings, SettingsBuilder};
pub use error::{Error, Result};

/// HTTP/2 connection preface that must be sent by clients
///
/// From RFC 7540 Section 3.5:
/// "PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n"
pub const CONNECTION_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

/// Default initial window size (65535 bytes)
pub const DEFAULT_INITIAL_WINDOW_SIZE: u32 = 65535;

/// Default maximum frame size (16384 bytes)
pub const DEFAULT_MAX_FRAME_SIZE: u32 = 16384;

/// Default header table size (4096 bytes)
pub const DEFAULT_HEADER_TABLE_SIZE: u32 = 4096;

/// Maximum stream ID value (2^31 - 1)
pub const MAX_STREAM_ID: u32 = 0x7FFFFFFF;

/// Stream ID 0 (connection-level)
pub const CONNECTION_STREAM_ID: u32 = 0;
