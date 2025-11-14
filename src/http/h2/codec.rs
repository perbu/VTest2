//! HTTP/2 frame encoding and decoding
//!
//! This module provides low-level frame encoding/decoding with full control
//! over frame construction, allowing intentionally malformed frames for testing.
//!
//! This is a direct port of the C implementation's frame handling.

use super::error::{Error, ErrorCode, Result};
use super::frames::*;
use super::settings::SettingsParameter;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::io::{self, Read, Write};

/// HTTP/2 frame header size (9 bytes)
pub const FRAME_HEADER_SIZE: usize = 9;

/// Maximum frame payload size (16MB - 1)
pub const MAX_FRAME_SIZE: usize = 0x00FFFFFF;

/// Frame codec for encoding/decoding HTTP/2 frames
pub struct FrameCodec {
    /// Buffer for reading
    read_buffer: BytesMut,
}

impl FrameCodec {
    /// Create a new frame codec
    pub fn new() -> Self {
        FrameCodec {
            read_buffer: BytesMut::with_capacity(4096),
        }
    }

    /// Encode a frame header into a buffer
    ///
    /// This directly ports the C function writeFrameHeader
    pub fn encode_header(frame_type: FrameType, flags: FrameFlags, stream_id: u32, length: usize) -> [u8; FRAME_HEADER_SIZE] {
        let mut header = [0u8; FRAME_HEADER_SIZE];

        // Length (24 bits, big-endian)
        header[0] = ((length >> 16) & 0xFF) as u8;
        header[1] = ((length >> 8) & 0xFF) as u8;
        header[2] = (length & 0xFF) as u8;

        // Type (8 bits)
        header[3] = frame_type.as_u8();

        // Flags (8 bits)
        header[4] = flags.as_u8();

        // Stream ID (31 bits, big-endian, reserved bit is 0)
        let stream_id = stream_id & 0x7FFFFFFF; // Mask reserved bit
        header[5] = ((stream_id >> 24) & 0xFF) as u8;
        header[6] = ((stream_id >> 16) & 0xFF) as u8;
        header[7] = ((stream_id >> 8) & 0xFF) as u8;
        header[8] = (stream_id & 0xFF) as u8;

        header
    }

    /// Decode a frame header from bytes
    ///
    /// This directly ports the C function readFrameHeader
    pub fn decode_header(bytes: &[u8; FRAME_HEADER_SIZE]) -> (FrameType, FrameFlags, u32, usize) {
        // Length (24 bits, big-endian)
        let length = ((bytes[0] as usize) << 16)
            | ((bytes[1] as usize) << 8)
            | (bytes[2] as usize);

        // Type (8 bits)
        let frame_type = FrameType::from_u8(bytes[3]).unwrap_or(FrameType::Data);

        // Flags (8 bits)
        let flags = FrameFlags::from_u8(bytes[4]);

        // Stream ID (31 bits, ignore reserved bit)
        let stream_id = ((bytes[5] as u32 & 0x7F) << 24)  // Mask reserved bit
            | ((bytes[6] as u32) << 16)
            | ((bytes[7] as u32) << 8)
            | (bytes[8] as u32);

        (frame_type, flags, stream_id, length)
    }

    /// Encode a DATA frame
    pub fn encode_data_frame(frame: &DataFrame) -> Bytes {
        let mut buf = BytesMut::new();

        // Calculate payload size
        let mut payload_len = frame.data.len();
        let mut flags = FrameFlags::empty();

        if frame.end_stream {
            flags.set(FrameFlags::END_STREAM);
        }

        // Add padding if requested
        let padding_len = if let Some(pad_len) = frame.padding {
            flags.set(FrameFlags::PADDED);
            payload_len += 1 + pad_len as usize; // 1 byte for length + padding
            pad_len
        } else {
            0
        };

        // Write frame header
        let header = Self::encode_header(FrameType::Data, flags, frame.stream_id, payload_len);
        buf.put_slice(&header);

        // Write padding length if padded
        if frame.padding.is_some() {
            buf.put_u8(padding_len);
        }

        // Write data
        buf.put_slice(&frame.data);

        // Write padding
        if padding_len > 0 {
            buf.put_bytes(0, padding_len as usize);
        }

        buf.freeze()
    }

    /// Encode a HEADERS frame
    pub fn encode_headers_frame(frame: &HeadersFrame) -> Bytes {
        let mut buf = BytesMut::new();

        let mut payload_len = frame.header_block.len();
        let mut flags = FrameFlags::empty();

        if frame.end_stream {
            flags.set(FrameFlags::END_STREAM);
        }
        if frame.end_headers {
            flags.set(FrameFlags::END_HEADERS);
        }

        // Add priority if present
        let has_priority = frame.priority.is_some();
        if has_priority {
            flags.set(FrameFlags::PRIORITY);
            payload_len += 5; // Priority is 5 bytes
        }

        // Add padding if requested
        let padding_len = if let Some(pad_len) = frame.padding {
            flags.set(FrameFlags::PADDED);
            payload_len += 1 + pad_len as usize;
            pad_len
        } else {
            0
        };

        // Write frame header
        let header = Self::encode_header(FrameType::Headers, flags, frame.stream_id, payload_len);
        buf.put_slice(&header);

        // Write padding length if padded
        if frame.padding.is_some() {
            buf.put_u8(padding_len);
        }

        // Write priority if present
        if let Some(priority) = &frame.priority {
            let mut dep = priority.stream_dependency;
            if priority.exclusive {
                dep |= 0x80000000; // Set exclusive bit
            }
            buf.put_u32(dep);
            buf.put_u8(priority.weight);
        }

        // Write header block
        buf.put_slice(&frame.header_block);

        // Write padding
        if padding_len > 0 {
            buf.put_bytes(0, padding_len as usize);
        }

        buf.freeze()
    }

    /// Encode a SETTINGS frame
    pub fn encode_settings_frame(frame: &SettingsFrame) -> Bytes {
        let mut buf = BytesMut::new();

        let flags = if frame.ack {
            FrameFlags::from_u8(FrameFlags::ACK)
        } else {
            FrameFlags::empty()
        };

        // Each setting is 6 bytes (2 byte ID + 4 byte value)
        let mut settings_data = BytesMut::new();

        if !frame.ack {
            let settings = &frame.settings;

            if let Some(val) = settings.header_table_size {
                settings_data.put_u16(SettingsParameter::HeaderTableSize.as_u16());
                settings_data.put_u32(val);
            }
            if let Some(val) = settings.enable_push {
                settings_data.put_u16(SettingsParameter::EnablePush.as_u16());
                settings_data.put_u32(if val { 1 } else { 0 });
            }
            if let Some(val) = settings.max_concurrent_streams {
                settings_data.put_u16(SettingsParameter::MaxConcurrentStreams.as_u16());
                settings_data.put_u32(val);
            }
            if let Some(val) = settings.initial_window_size {
                settings_data.put_u16(SettingsParameter::InitialWindowSize.as_u16());
                settings_data.put_u32(val);
            }
            if let Some(val) = settings.max_frame_size {
                settings_data.put_u16(SettingsParameter::MaxFrameSize.as_u16());
                settings_data.put_u32(val);
            }
            if let Some(val) = settings.max_header_list_size {
                settings_data.put_u16(SettingsParameter::MaxHeaderListSize.as_u16());
                settings_data.put_u32(val);
            }
            if let Some(val) = settings.enable_connect_protocol {
                settings_data.put_u16(SettingsParameter::EnableConnectProtocol.as_u16());
                settings_data.put_u32(if val { 1 } else { 0 });
            }
            if let Some(val) = settings.no_rfc7540_priorities {
                settings_data.put_u16(SettingsParameter::NoRfc7540Priorities.as_u16());
                settings_data.put_u32(if val { 1 } else { 0 });
            }
        }

        // Write frame header (stream ID must be 0 for SETTINGS)
        let header = Self::encode_header(FrameType::Settings, flags, 0, settings_data.len());
        buf.put_slice(&header);
        buf.put_slice(&settings_data);

        buf.freeze()
    }

    /// Encode a PING frame
    pub fn encode_ping_frame(frame: &PingFrame) -> Bytes {
        let mut buf = BytesMut::new();

        let flags = if frame.ack {
            FrameFlags::from_u8(FrameFlags::ACK)
        } else {
            FrameFlags::empty()
        };

        // Write frame header (stream ID must be 0 for PING, payload is always 8 bytes)
        let header = Self::encode_header(FrameType::Ping, flags, 0, 8);
        buf.put_slice(&header);
        buf.put_slice(&frame.data);

        buf.freeze()
    }

    /// Encode a GOAWAY frame
    pub fn encode_goaway_frame(frame: &GoawayFrame) -> Bytes {
        let mut buf = BytesMut::new();

        let payload_len = 8 + frame.debug_data.len(); // 4 bytes stream ID + 4 bytes error code + debug data

        // Write frame header (stream ID must be 0 for GOAWAY)
        let header = Self::encode_header(FrameType::Goaway, FrameFlags::empty(), 0, payload_len);
        buf.put_slice(&header);

        // Write last stream ID
        buf.put_u32(frame.last_stream_id & 0x7FFFFFFF);

        // Write error code
        buf.put_u32(frame.error_code.as_u32());

        // Write debug data
        buf.put_slice(&frame.debug_data);

        buf.freeze()
    }

    /// Encode a WINDOW_UPDATE frame
    pub fn encode_window_update_frame(frame: &WindowUpdateFrame) -> Bytes {
        let mut buf = BytesMut::new();

        // Write frame header (payload is always 4 bytes)
        let header = Self::encode_header(FrameType::WindowUpdate, FrameFlags::empty(), frame.stream_id, 4);
        buf.put_slice(&header);

        // Write window size increment (reserved bit must be 0)
        buf.put_u32(frame.size_increment & 0x7FFFFFFF);

        buf.freeze()
    }

    /// Encode a RST_STREAM frame
    pub fn encode_rst_stream_frame(frame: &RstStreamFrame) -> Bytes {
        let mut buf = BytesMut::new();

        // Write frame header (payload is always 4 bytes)
        let header = Self::encode_header(FrameType::RstStream, FrameFlags::empty(), frame.stream_id, 4);
        buf.put_slice(&header);

        // Write error code
        buf.put_u32(frame.error_code.as_u32());

        buf.freeze()
    }

    /// Encode a PRIORITY frame
    pub fn encode_priority_frame(frame: &PriorityFrame) -> Bytes {
        let mut buf = BytesMut::new();

        // Write frame header (payload is always 5 bytes)
        let header = Self::encode_header(FrameType::Priority, FrameFlags::empty(), frame.stream_id, 5);
        buf.put_slice(&header);

        // Write priority
        let mut dep = frame.priority.stream_dependency;
        if frame.priority.exclusive {
            dep |= 0x80000000;
        }
        buf.put_u32(dep);
        buf.put_u8(frame.priority.weight);

        buf.freeze()
    }

    /// Write a frame to a writer (generic over any Write)
    pub fn write_frame<W: Write>(writer: &mut W, frame_data: &[u8]) -> io::Result<()> {
        writer.write_all(frame_data)?;
        writer.flush()?;
        Ok(())
    }

    /// Read a frame from a buffer reader (works with any Read impl)
    pub fn read_frame<R: Read>(reader: &mut R) -> io::Result<(FrameType, FrameFlags, u32, Bytes)> {
        // Read frame header
        let mut header = [0u8; FRAME_HEADER_SIZE];
        reader.read_exact(&mut header)?;

        let (frame_type, flags, stream_id, payload_len) = Self::decode_header(&header);

        // Validate payload length
        if payload_len > MAX_FRAME_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Frame payload too large: {}", payload_len),
            ));
        }

        // Read payload
        let mut payload = vec![0u8; payload_len];
        if payload_len > 0 {
            reader.read_exact(&mut payload)?;
        }

        Ok((frame_type, flags, stream_id, Bytes::from(payload)))
    }

    /// Read a frame from an HttpSession (helper for H2Client)
    pub fn read_frame_from_session<S: crate::http::SessionOps>(
        session: &mut crate::http::HttpSession<S>
    ) -> io::Result<(FrameType, FrameFlags, u32, Bytes)> {
        // Read frame header
        let mut header = [0u8; FRAME_HEADER_SIZE];
        let mut read = 0;
        while read < FRAME_HEADER_SIZE {
            let n = session.read(&mut header[read..]).map_err(|e| match e {
                crate::http::Error::Io(io_err) => io_err,
                other => io::Error::new(io::ErrorKind::Other, other.to_string()),
            })?;
            if n == 0 {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Connection closed"));
            }
            read += n;
        }

        let (frame_type, flags, stream_id, payload_len) = Self::decode_header(&header);

        // Validate payload length
        if payload_len > MAX_FRAME_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Frame payload too large: {}", payload_len),
            ));
        }

        // Read payload
        let mut payload = vec![0u8; payload_len];
        if payload_len > 0 {
            let mut read = 0;
            while read < payload_len {
                let n = session.read(&mut payload[read..]).map_err(|e| match e {
                    crate::http::Error::Io(io_err) => io_err,
                    other => io::Error::new(io::ErrorKind::Other, other.to_string()),
                })?;
                if n == 0 {
                    return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Connection closed"));
                }
                read += n;
            }
        }

        Ok((frame_type, flags, stream_id, Bytes::from(payload)))
    }
}

impl Default for FrameCodec {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_header() {
        let frame_type = FrameType::Headers;
        let flags = FrameFlags::from_u8(FrameFlags::END_STREAM | FrameFlags::END_HEADERS);
        let stream_id = 42;
        let length = 1234;

        let header = FrameCodec::encode_header(frame_type, flags, stream_id, length);
        let (decoded_type, decoded_flags, decoded_id, decoded_len) = FrameCodec::decode_header(&header);

        assert_eq!(decoded_type, frame_type);
        assert_eq!(decoded_flags.as_u8(), flags.as_u8());
        assert_eq!(decoded_id, stream_id);
        assert_eq!(decoded_len, length);
    }

    #[test]
    fn test_encode_data_frame() {
        let frame = DataFrame::new(1, Bytes::from("Hello"), true);
        let encoded = FrameCodec::encode_data_frame(&frame);

        // Check frame header
        assert_eq!(encoded[0..3], [0, 0, 5]); // Length = 5
        assert_eq!(encoded[3], FrameType::Data.as_u8());
        assert_eq!(encoded[4], FrameFlags::END_STREAM);
        assert_eq!(&encoded[5..9], &[0, 0, 0, 1]); // Stream ID = 1

        // Check payload
        assert_eq!(&encoded[9..], b"Hello");
    }

    #[test]
    fn test_encode_data_frame_with_padding() {
        let frame = DataFrame::new(1, Bytes::from("Hi"), false).with_padding(10);
        let encoded = FrameCodec::encode_data_frame(&frame);

        // Length should be: 1 (pad length) + 2 (data) + 10 (padding) = 13
        assert_eq!(encoded[0..3], [0, 0, 13]);
        assert_eq!(encoded[4] & FrameFlags::PADDED, FrameFlags::PADDED);

        // Padding length field
        assert_eq!(encoded[9], 10);

        // Data
        assert_eq!(&encoded[10..12], b"Hi");

        // Padding (all zeros)
        assert_eq!(&encoded[12..22], &[0u8; 10]);
    }

    #[test]
    fn test_encode_settings_frame() {
        let settings = SettingsBuilder::new()
            .header_table_size(8192)
            .enable_push(false)
            .initial_window_size(65535)
            .build()
            .unwrap();

        let frame = SettingsFrame::new(settings);
        let encoded = FrameCodec::encode_settings_frame(&frame);

        // Frame header
        assert_eq!(encoded[3], FrameType::Settings.as_u8());
        assert_eq!(&encoded[5..9], &[0, 0, 0, 0]); // Stream ID must be 0

        // Should have 3 settings * 6 bytes = 18 bytes payload
        assert_eq!(encoded[0..3], [0, 0, 18]);
    }

    #[test]
    fn test_encode_settings_ack() {
        let frame = SettingsFrame::ack();
        let encoded = FrameCodec::encode_settings_frame(&frame);

        // Length should be 0 for ACK
        assert_eq!(encoded[0..3], [0, 0, 0]);
        assert_eq!(encoded[4], FrameFlags::ACK);
    }

    #[test]
    fn test_encode_ping_frame() {
        let data = [1, 2, 3, 4, 5, 6, 7, 8];
        let frame = PingFrame::new(data);
        let encoded = FrameCodec::encode_ping_frame(&frame);

        // Length should be 8
        assert_eq!(encoded[0..3], [0, 0, 8]);
        assert_eq!(encoded[3], FrameType::Ping.as_u8());
        assert_eq!(&encoded[9..17], &data);
    }

    #[test]
    fn test_encode_window_update() {
        let frame = WindowUpdateFrame::new(42, 1000);
        let encoded = FrameCodec::encode_window_update_frame(&frame);

        // Length should be 4
        assert_eq!(encoded[0..3], [0, 0, 4]);
        assert_eq!(encoded[3], FrameType::WindowUpdate.as_u8());

        // Stream ID
        assert_eq!(&encoded[5..9], &[0, 0, 0, 42]);

        // Window size increment
        let increment = u32::from_be_bytes([encoded[9], encoded[10], encoded[11], encoded[12]]);
        assert_eq!(increment, 1000);
    }
}
