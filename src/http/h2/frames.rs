//! HTTP/2 frame types and utilities
//!
//! This module defines the frame types specified in RFC 7540 Section 6.

use super::error::ErrorCode;
use super::settings::Settings;
use bytes::Bytes;
use std::fmt;

/// HTTP/2 frame types (RFC 7540 Section 6)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameType {
    /// DATA frame (0x0) - Conveys arbitrary, variable-length sequences of octets
    Data = 0x0,
    /// HEADERS frame (0x1) - Opens a stream and carries header block fragment
    Headers = 0x1,
    /// PRIORITY frame (0x2) - Specifies sender-advised priority of a stream
    Priority = 0x2,
    /// RST_STREAM frame (0x3) - Allows immediate termination of a stream
    RstStream = 0x3,
    /// SETTINGS frame (0x4) - Conveys configuration parameters
    Settings = 0x4,
    /// PUSH_PROMISE frame (0x5) - Used to notify peer of intent to initiate stream
    PushPromise = 0x5,
    /// PING frame (0x6) - Mechanism for measuring round-trip time
    Ping = 0x6,
    /// GOAWAY frame (0x7) - Initiates shutdown of connection
    Goaway = 0x7,
    /// WINDOW_UPDATE frame (0x8) - Implements flow control
    WindowUpdate = 0x8,
    /// CONTINUATION frame (0x9) - Continues sequence of header block fragments
    Continuation = 0x9,
}

impl FrameType {
    /// Convert frame type to u8
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Create frame type from u8
    pub fn from_u8(byte: u8) -> Option<Self> {
        match byte {
            0x0 => Some(FrameType::Data),
            0x1 => Some(FrameType::Headers),
            0x2 => Some(FrameType::Priority),
            0x3 => Some(FrameType::RstStream),
            0x4 => Some(FrameType::Settings),
            0x5 => Some(FrameType::PushPromise),
            0x6 => Some(FrameType::Ping),
            0x7 => Some(FrameType::Goaway),
            0x8 => Some(FrameType::WindowUpdate),
            0x9 => Some(FrameType::Continuation),
            _ => None,
        }
    }

    /// Get frame type name
    pub fn name(&self) -> &'static str {
        match self {
            FrameType::Data => "DATA",
            FrameType::Headers => "HEADERS",
            FrameType::Priority => "PRIORITY",
            FrameType::RstStream => "RST_STREAM",
            FrameType::Settings => "SETTINGS",
            FrameType::PushPromise => "PUSH_PROMISE",
            FrameType::Ping => "PING",
            FrameType::Goaway => "GOAWAY",
            FrameType::WindowUpdate => "WINDOW_UPDATE",
            FrameType::Continuation => "CONTINUATION",
        }
    }
}

impl fmt::Display for FrameType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (0x{:x})", self.name(), self.as_u8())
    }
}

/// HTTP/2 frame flags
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameFlags(u8);

impl FrameFlags {
    /// Create empty flags
    pub fn empty() -> Self {
        FrameFlags(0)
    }

    /// Create from u8
    pub fn from_u8(flags: u8) -> Self {
        FrameFlags(flags)
    }

    /// Get raw u8 value
    pub fn as_u8(&self) -> u8 {
        self.0
    }

    /// Set a flag
    pub fn set(&mut self, flag: u8) {
        self.0 |= flag;
    }

    /// Check if a flag is set
    pub fn is_set(&self, flag: u8) -> bool {
        (self.0 & flag) != 0
    }

    // Common flags

    /// END_STREAM flag (0x1)
    pub const END_STREAM: u8 = 0x1;

    /// ACK flag (0x1) - used for SETTINGS and PING
    pub const ACK: u8 = 0x1;

    /// END_HEADERS flag (0x4)
    pub const END_HEADERS: u8 = 0x4;

    /// PADDED flag (0x8)
    pub const PADDED: u8 = 0x8;

    /// PRIORITY flag (0x20)
    pub const PRIORITY: u8 = 0x20;

    /// Check if END_STREAM is set
    pub fn is_end_stream(&self) -> bool {
        self.is_set(Self::END_STREAM)
    }

    /// Check if ACK is set
    pub fn is_ack(&self) -> bool {
        self.is_set(Self::ACK)
    }

    /// Check if END_HEADERS is set
    pub fn is_end_headers(&self) -> bool {
        self.is_set(Self::END_HEADERS)
    }

    /// Check if PADDED is set
    pub fn is_padded(&self) -> bool {
        self.is_set(Self::PADDED)
    }

    /// Check if PRIORITY is set
    pub fn is_priority(&self) -> bool {
        self.is_set(Self::PRIORITY)
    }
}

/// Generic HTTP/2 frame
#[derive(Debug, Clone)]
pub struct Frame {
    /// Frame type
    pub frame_type: FrameType,
    /// Frame flags
    pub flags: FrameFlags,
    /// Stream ID
    pub stream_id: u32,
    /// Frame payload
    pub payload: Bytes,
}

impl Frame {
    /// Create a new frame
    pub fn new(frame_type: FrameType, flags: FrameFlags, stream_id: u32, payload: Bytes) -> Self {
        Frame {
            frame_type,
            flags,
            stream_id,
            payload,
        }
    }

    /// Get payload size
    pub fn payload_len(&self) -> usize {
        self.payload.len()
    }
}

/// DATA frame (RFC 7540 Section 6.1)
#[derive(Debug, Clone)]
pub struct DataFrame {
    /// Stream ID
    pub stream_id: u32,
    /// Data payload
    pub data: Bytes,
    /// END_STREAM flag
    pub end_stream: bool,
    /// Padding length (if PADDED flag is set)
    pub padding: Option<u8>,
}

impl DataFrame {
    /// Create a new DATA frame
    pub fn new(stream_id: u32, data: Bytes, end_stream: bool) -> Self {
        DataFrame {
            stream_id,
            data,
            end_stream,
            padding: None,
        }
    }

    /// Set padding
    pub fn with_padding(mut self, padding: u8) -> Self {
        self.padding = Some(padding);
        self
    }

    /// Get total frame size including padding
    pub fn frame_size(&self) -> usize {
        let mut size = self.data.len();
        if let Some(pad_len) = self.padding {
            size += 1 + pad_len as usize; // 1 byte for pad length field + padding
        }
        size
    }
}

/// HEADERS frame (RFC 7540 Section 6.2)
#[derive(Debug, Clone)]
pub struct HeadersFrame {
    /// Stream ID
    pub stream_id: u32,
    /// Header block fragment
    pub header_block: Bytes,
    /// END_STREAM flag
    pub end_stream: bool,
    /// END_HEADERS flag
    pub end_headers: bool,
    /// Priority information (if PRIORITY flag is set)
    pub priority: Option<PrioritySpec>,
    /// Padding length (if PADDED flag is set)
    pub padding: Option<u8>,
}

impl HeadersFrame {
    /// Create a new HEADERS frame
    pub fn new(stream_id: u32, header_block: Bytes, end_stream: bool, end_headers: bool) -> Self {
        HeadersFrame {
            stream_id,
            header_block,
            end_stream,
            end_headers,
            priority: None,
            padding: None,
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: PrioritySpec) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Set padding
    pub fn with_padding(mut self, padding: u8) -> Self {
        self.padding = Some(padding);
        self
    }
}

/// Priority specification (RFC 7540 Section 6.3)
#[derive(Debug, Clone, Copy)]
pub struct PrioritySpec {
    /// Stream dependency
    pub stream_dependency: u32,
    /// Exclusive flag
    pub exclusive: bool,
    /// Weight (1-256)
    pub weight: u8,
}

impl PrioritySpec {
    /// Create a new priority specification
    pub fn new(stream_dependency: u32, exclusive: bool, weight: u8) -> Self {
        PrioritySpec {
            stream_dependency,
            exclusive,
            weight,
        }
    }
}

/// PRIORITY frame (RFC 7540 Section 6.3)
#[derive(Debug, Clone, Copy)]
pub struct PriorityFrame {
    /// Stream ID
    pub stream_id: u32,
    /// Priority specification
    pub priority: PrioritySpec,
}

/// RST_STREAM frame (RFC 7540 Section 6.4)
#[derive(Debug, Clone, Copy)]
pub struct RstStreamFrame {
    /// Stream ID
    pub stream_id: u32,
    /// Error code
    pub error_code: ErrorCode,
}

/// SETTINGS frame (RFC 7540 Section 6.5)
#[derive(Debug, Clone)]
pub struct SettingsFrame {
    /// ACK flag
    pub ack: bool,
    /// Settings parameters
    pub settings: Settings,
}

impl SettingsFrame {
    /// Create a new SETTINGS frame
    pub fn new(settings: Settings) -> Self {
        SettingsFrame {
            ack: false,
            settings,
        }
    }

    /// Create a SETTINGS ACK frame
    pub fn ack() -> Self {
        SettingsFrame {
            ack: true,
            settings: Settings::default(),
        }
    }
}

/// PUSH_PROMISE frame (RFC 7540 Section 6.6)
#[derive(Debug, Clone)]
pub struct PushPromiseFrame {
    /// Stream ID
    pub stream_id: u32,
    /// Promised stream ID
    pub promised_stream_id: u32,
    /// Header block fragment
    pub header_block: Bytes,
    /// END_HEADERS flag
    pub end_headers: bool,
    /// Padding length (if PADDED flag is set)
    pub padding: Option<u8>,
}

/// PING frame (RFC 7540 Section 6.7)
#[derive(Debug, Clone, Copy)]
pub struct PingFrame {
    /// ACK flag
    pub ack: bool,
    /// Opaque data (8 bytes)
    pub data: [u8; 8],
}

impl PingFrame {
    /// Create a new PING frame
    pub fn new(data: [u8; 8]) -> Self {
        PingFrame { ack: false, data }
    }

    /// Create a PING ACK frame
    pub fn ack(data: [u8; 8]) -> Self {
        PingFrame { ack: true, data }
    }
}

/// GOAWAY frame (RFC 7540 Section 6.8)
#[derive(Debug, Clone)]
pub struct GoawayFrame {
    /// Last stream ID
    pub last_stream_id: u32,
    /// Error code
    pub error_code: ErrorCode,
    /// Debug data
    pub debug_data: Bytes,
}

impl GoawayFrame {
    /// Create a new GOAWAY frame
    pub fn new(last_stream_id: u32, error_code: ErrorCode, debug_data: Bytes) -> Self {
        GoawayFrame {
            last_stream_id,
            error_code,
            debug_data,
        }
    }
}

/// WINDOW_UPDATE frame (RFC 7540 Section 6.9)
#[derive(Debug, Clone, Copy)]
pub struct WindowUpdateFrame {
    /// Stream ID (0 for connection-level)
    pub stream_id: u32,
    /// Window size increment
    pub size_increment: u32,
}

impl WindowUpdateFrame {
    /// Create a new WINDOW_UPDATE frame
    pub fn new(stream_id: u32, size_increment: u32) -> Self {
        WindowUpdateFrame {
            stream_id,
            size_increment,
        }
    }
}

/// CONTINUATION frame (RFC 7540 Section 6.10)
#[derive(Debug, Clone)]
pub struct ContinuationFrame {
    /// Stream ID
    pub stream_id: u32,
    /// Header block fragment
    pub header_block: Bytes,
    /// END_HEADERS flag
    pub end_headers: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_type_conversion() {
        assert_eq!(FrameType::Data.as_u8(), 0x0);
        assert_eq!(FrameType::Headers.as_u8(), 0x1);
        assert_eq!(FrameType::Continuation.as_u8(), 0x9);

        assert_eq!(FrameType::from_u8(0x0), Some(FrameType::Data));
        assert_eq!(FrameType::from_u8(0x9), Some(FrameType::Continuation));
        assert_eq!(FrameType::from_u8(0xff), None);
    }

    #[test]
    fn test_frame_type_name() {
        assert_eq!(FrameType::Data.name(), "DATA");
        assert_eq!(FrameType::Headers.name(), "HEADERS");
        assert_eq!(FrameType::Settings.name(), "SETTINGS");
    }

    #[test]
    fn test_frame_flags() {
        let mut flags = FrameFlags::empty();
        assert!(!flags.is_end_stream());

        flags.set(FrameFlags::END_STREAM);
        assert!(flags.is_end_stream());
        assert!(!flags.is_end_headers());

        flags.set(FrameFlags::END_HEADERS);
        assert!(flags.is_end_stream());
        assert!(flags.is_end_headers());
    }

    #[test]
    fn test_data_frame() {
        let data = Bytes::from("Hello");
        let frame = DataFrame::new(1, data.clone(), true);

        assert_eq!(frame.stream_id, 1);
        assert_eq!(frame.data, data);
        assert!(frame.end_stream);
        assert_eq!(frame.padding, None);
        assert_eq!(frame.frame_size(), 5);

        let frame_with_padding = frame.with_padding(10);
        assert_eq!(frame_with_padding.padding, Some(10));
        assert_eq!(frame_with_padding.frame_size(), 16); // 5 + 1 + 10
    }

    #[test]
    fn test_settings_frame() {
        let settings = Settings::default();
        let frame = SettingsFrame::new(settings);
        assert!(!frame.ack);

        let ack_frame = SettingsFrame::ack();
        assert!(ack_frame.ack);
    }

    #[test]
    fn test_ping_frame() {
        let data = [1, 2, 3, 4, 5, 6, 7, 8];
        let ping = PingFrame::new(data);
        assert!(!ping.ack);
        assert_eq!(ping.data, data);

        let pong = PingFrame::ack(data);
        assert!(pong.ack);
        assert_eq!(pong.data, data);
    }
}
