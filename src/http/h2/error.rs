//! HTTP/2 error types
//!
//! This module defines error types for HTTP/2 operations, mapping to
//! the error codes defined in RFC 7540 Section 7.

use std::fmt;

/// HTTP/2 errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// HTTP error from underlying HTTP layer
    #[error("HTTP error: {0}")]
    Http(#[from] crate::http::Error),

    /// Protocol error detected (RFC 7540 Section 7 - Error code 0x1)
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Internal error (RFC 7540 Section 7 - Error code 0x2)
    #[error("Internal error: {0}")]
    Internal(String),

    /// Flow control error (RFC 7540 Section 7 - Error code 0x3)
    #[error("Flow control error: {0}")]
    FlowControl(String),

    /// Settings timeout (RFC 7540 Section 7 - Error code 0x4)
    #[error("Settings timeout")]
    SettingsTimeout,

    /// Stream closed (RFC 7540 Section 7 - Error code 0x5)
    #[error("Stream closed: {0}")]
    StreamClosed(u32),

    /// Frame size error (RFC 7540 Section 7 - Error code 0x6)
    #[error("Frame size error: {0}")]
    FrameSize(String),

    /// Refused stream (RFC 7540 Section 7 - Error code 0x7)
    #[error("Refused stream: {0}")]
    RefusedStream(u32),

    /// Stream cancelled (RFC 7540 Section 7 - Error code 0x8)
    #[error("Stream cancelled: {0}")]
    Cancel(u32),

    /// Compression error (RFC 7540 Section 7 - Error code 0x9)
    #[error("Compression error: {0}")]
    Compression(String),

    /// Connect error (RFC 7540 Section 7 - Error code 0xa)
    #[error("Connect error: {0}")]
    Connect(String),

    /// Enhance your calm (RFC 7540 Section 7 - Error code 0xb)
    #[error("Enhance your calm: {0}")]
    EnhanceYourCalm(String),

    /// Inadequate security (RFC 7540 Section 7 - Error code 0xc)
    #[error("Inadequate security: {0}")]
    InadequateSecurity(String),

    /// HTTP/1.1 required (RFC 7540 Section 7 - Error code 0xd)
    #[error("HTTP/1.1 required")]
    Http11Required,

    /// Invalid stream ID
    #[error("Invalid stream ID: {0}")]
    InvalidStreamId(u32),

    /// Invalid frame type
    #[error("Invalid frame type: {0}")]
    InvalidFrameType(u8),

    /// Connection not ready
    #[error("Connection not ready")]
    NotReady,

    /// ALPN negotiation failed
    #[error("ALPN negotiation failed: expected h2, got {0:?}")]
    AlpnFailed(Option<Vec<u8>>),

    /// Timeout waiting for operation
    #[error("Timeout")]
    Timeout,

    /// Stream not found
    #[error("Stream not found: {0}")]
    StreamNotFound(u32),

    /// Too many streams
    #[error("Too many concurrent streams")]
    TooManyStreams,

    /// Invalid settings value
    #[error("Invalid settings value: {0}")]
    InvalidSettings(String),

    /// Connection closed
    #[error("Connection closed")]
    ConnectionClosed,

    /// Missing connection preface
    #[error("Missing connection preface")]
    MissingPreface,

    /// Invalid header
    #[error("Invalid header: {0}")]
    InvalidHeader(String),
}

/// HTTP/2 error codes as defined in RFC 7540 Section 7
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ErrorCode {
    /// Graceful shutdown
    NoError = 0x0,
    /// Protocol error detected
    ProtocolError = 0x1,
    /// Implementation fault
    InternalError = 0x2,
    /// Flow-control limits exceeded
    FlowControlError = 0x3,
    /// Settings not acknowledged
    SettingsTimeout = 0x4,
    /// Frame received for closed stream
    StreamClosed = 0x5,
    /// Frame size incorrect
    FrameSizeError = 0x6,
    /// Stream not processed
    RefusedStream = 0x7,
    /// Stream cancelled
    Cancel = 0x8,
    /// Compression state not updated
    CompressionError = 0x9,
    /// TCP connection error for CONNECT method
    ConnectError = 0xa,
    /// Processing capacity exceeded
    EnhanceYourCalm = 0xb,
    /// Negotiated TLS parameters not acceptable
    InadequateSecurity = 0xc,
    /// Use HTTP/1.1 for the request
    Http11Required = 0xd,
}

impl ErrorCode {
    /// Convert error code to u32
    pub fn as_u32(self) -> u32 {
        self as u32
    }

    /// Create error code from u32
    pub fn from_u32(code: u32) -> Option<Self> {
        match code {
            0x0 => Some(ErrorCode::NoError),
            0x1 => Some(ErrorCode::ProtocolError),
            0x2 => Some(ErrorCode::InternalError),
            0x3 => Some(ErrorCode::FlowControlError),
            0x4 => Some(ErrorCode::SettingsTimeout),
            0x5 => Some(ErrorCode::StreamClosed),
            0x6 => Some(ErrorCode::FrameSizeError),
            0x7 => Some(ErrorCode::RefusedStream),
            0x8 => Some(ErrorCode::Cancel),
            0x9 => Some(ErrorCode::CompressionError),
            0xa => Some(ErrorCode::ConnectError),
            0xb => Some(ErrorCode::EnhanceYourCalm),
            0xc => Some(ErrorCode::InadequateSecurity),
            0xd => Some(ErrorCode::Http11Required),
            _ => None,
        }
    }

    /// Get error name
    pub fn name(&self) -> &'static str {
        match self {
            ErrorCode::NoError => "NO_ERROR",
            ErrorCode::ProtocolError => "PROTOCOL_ERROR",
            ErrorCode::InternalError => "INTERNAL_ERROR",
            ErrorCode::FlowControlError => "FLOW_CONTROL_ERROR",
            ErrorCode::SettingsTimeout => "SETTINGS_TIMEOUT",
            ErrorCode::StreamClosed => "STREAM_CLOSED",
            ErrorCode::FrameSizeError => "FRAME_SIZE_ERROR",
            ErrorCode::RefusedStream => "REFUSED_STREAM",
            ErrorCode::Cancel => "CANCEL",
            ErrorCode::CompressionError => "COMPRESSION_ERROR",
            ErrorCode::ConnectError => "CONNECT_ERROR",
            ErrorCode::EnhanceYourCalm => "ENHANCE_YOUR_CALM",
            ErrorCode::InadequateSecurity => "INADEQUATE_SECURITY",
            ErrorCode::Http11Required => "HTTP_1_1_REQUIRED",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (0x{:x})", self.name(), self.as_u32())
    }
}

/// Result type for HTTP/2 operations
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_conversion() {
        assert_eq!(ErrorCode::NoError.as_u32(), 0x0);
        assert_eq!(ErrorCode::ProtocolError.as_u32(), 0x1);
        assert_eq!(ErrorCode::Http11Required.as_u32(), 0xd);

        assert_eq!(ErrorCode::from_u32(0x0), Some(ErrorCode::NoError));
        assert_eq!(ErrorCode::from_u32(0x1), Some(ErrorCode::ProtocolError));
        assert_eq!(ErrorCode::from_u32(0xff), None);
    }

    #[test]
    fn test_error_code_name() {
        assert_eq!(ErrorCode::NoError.name(), "NO_ERROR");
        assert_eq!(ErrorCode::ProtocolError.name(), "PROTOCOL_ERROR");
        assert_eq!(ErrorCode::FlowControlError.name(), "FLOW_CONTROL_ERROR");
    }

    #[test]
    fn test_error_display() {
        let err = Error::Protocol("test error".to_string());
        assert_eq!(err.to_string(), "Protocol error: test error");

        let err = Error::StreamClosed(42);
        assert_eq!(err.to_string(), "Stream closed: 42");
    }
}
