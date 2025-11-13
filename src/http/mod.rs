//! HTTP/1.1 implementation for VTest2
//!
//! This module provides HTTP/1.1 client and server functionality for testing.
//! It's a port of the C vtc_http.c module with idiomatic Rust patterns.
//!
//! # Architecture
//!
//! The HTTP layer uses a session operations abstraction pattern that allows
//! seamless switching between plain TCP and TLS connections:
//!
//! - `SessionOps` trait defines operations (poll, read, write, close)
//! - `HttpSession` contains operation pointers that default to plain FD operations
//! - All HTTP I/O code is transparent to the underlying transport
//!
//! # Examples
//!
//! ```no_run
//! use vtest2::http::{HttpClient, HttpRequest, Method};
//! use std::net::TcpStream;
//!
//! // Create client from TCP stream
//! let stream = TcpStream::connect("127.0.0.1:8080").unwrap();
//! let mut client = HttpClient::new(stream);
//!
//! // Send request
//! let request = HttpRequest::builder()
//!     .method(Method::Get)
//!     .uri("/")
//!     .header("Host", "localhost")
//!     .build();
//! client.send_request(&request).unwrap();
//!
//! // Receive response
//! let response = client.receive_response().unwrap();
//! assert_eq!(response.status().code(), 200);
//! ```

pub mod client;
pub mod headers;
pub mod message;
pub mod parser;
pub mod server;
pub mod session;
pub mod chunked;

pub use client::HttpClient;
pub use headers::Headers;
pub use message::{HttpRequest, HttpResponse, Method, Status, Version};
pub use parser::{RequestParser, ResponseParser};
pub use server::HttpServer;
pub use session::{SessionOps, HttpSession};

/// Result type for HTTP operations
pub type Result<T> = std::result::Result<T, Error>;

/// HTTP operation errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Network error: {0}")]
    Network(#[from] crate::net::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Invalid HTTP version: {0}")]
    InvalidVersion(String),

    #[error("Invalid HTTP method: {0}")]
    InvalidMethod(String),

    #[error("Invalid HTTP status: {0}")]
    InvalidStatus(String),

    #[error("Invalid header: {0}")]
    InvalidHeader(String),

    #[error("Invalid chunk size: {0}")]
    InvalidChunkSize(String),

    #[error("Incomplete message")]
    Incomplete,

    #[error("Timeout")]
    Timeout,

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Protocol error: {0}")]
    Protocol(String),
}

/// Maximum number of headers per message
pub const MAX_HEADERS: usize = 64;

/// Default HTTP port
pub const DEFAULT_HTTP_PORT: u16 = 80;

/// CRLF line ending
pub const CRLF: &str = "\r\n";
