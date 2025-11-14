//! HTTP/2 client implementation with low-level frame control
//!
//! This module provides HTTP/2 client functionality with full control over
//! frame construction, allowing testing of edge cases and protocol violations.

use super::codec::FrameCodec;
use super::error::{Error, ErrorCode, Result};
use super::flow_control::ConnectionFlowControl;
use super::frames::*;
use super::settings::{Settings, SettingsBuilder};
use super::stream::{StreamId, StreamManager};
use super::{CONNECTION_PREFACE, CONNECTION_STREAM_ID};
use crate::http::{SessionOps, HttpSession};
use bytes::Bytes;
use hpack::Encoder as HpackEncoder;
use std::collections::HashMap;

/// HTTP/2 client
///
/// Provides low-level control over HTTP/2 frame transmission, allowing
/// intentionally malformed frames for testing purposes.
pub struct H2Client<S: SessionOps> {
    /// HTTP session
    session: HttpSession<S>,
    /// Stream manager
    stream_manager: StreamManager,
    /// Connection-level flow control
    flow_control: ConnectionFlowControl,
    /// HPACK encoder
    hpack_encoder: HpackEncoder<'static>,
    /// HPACK decoder
    hpack_decoder: hpack::Decoder<'static>,
    /// Client settings
    local_settings: Settings,
    /// Remote (server) settings
    remote_settings: Settings,
    /// Connection established
    connected: bool,
}

impl<S: SessionOps> H2Client<S> {
    /// Create a new HTTP/2 client
    pub fn new(session: S) -> Result<Self> {
        H2ClientBuilder::new().build(session)
    }

    /// Perform HTTP/2 connection preface and settings exchange
    pub fn connect(&mut self) -> Result<()> {
        if self.connected {
            return Ok(());
        }

        // Send connection preface (RFC 7540 Section 3.5)
        self.session.write(CONNECTION_PREFACE)?;

        // Send initial SETTINGS frame
        let settings_frame = SettingsFrame::new(self.local_settings.clone());
        self.send_settings(&settings_frame)?;

        // Wait for server SETTINGS
        self.recv_settings()?;

        self.connected = true;
        Ok(())
    }

    /// Send a SETTINGS frame
    pub fn send_settings(&mut self, frame: &SettingsFrame) -> Result<()> {
        let encoded = FrameCodec::encode_settings_frame(frame);
        self.session.write(&encoded)?;
        Ok(())
    }

    /// Send a SETTINGS ACK
    pub fn send_settings_ack(&mut self) -> Result<()> {
        let frame = SettingsFrame::ack();
        self.send_settings(&frame)
    }

    /// Receive and process SETTINGS frame
    pub fn recv_settings(&mut self) -> Result<()> {
        let (frame_type, flags, stream_id, payload) = self.recv_frame()?;

        if frame_type != FrameType::Settings {
            return Err(Error::Protocol(format!(
                "Expected SETTINGS frame, got {:?}",
                frame_type
            )));
        }

        if stream_id != CONNECTION_STREAM_ID {
            return Err(Error::Protocol(
                "SETTINGS frame must have stream ID 0".to_string(),
            ));
        }

        // If it's an ACK, we're done
        if flags.is_ack() {
            return Ok(());
        }

        // Parse settings
        let mut settings = Settings::new();
        let mut pos = 0;
        while pos + 6 <= payload.len() {
            let id = u16::from_be_bytes([payload[pos], payload[pos + 1]]);
            let value = u32::from_be_bytes([
                payload[pos + 2],
                payload[pos + 3],
                payload[pos + 4],
                payload[pos + 5],
            ]);

            match id {
                0x1 => settings.header_table_size = Some(value),
                0x2 => settings.enable_push = Some(value != 0),
                0x3 => settings.max_concurrent_streams = Some(value),
                0x4 => settings.initial_window_size = Some(value),
                0x5 => settings.max_frame_size = Some(value),
                0x6 => settings.max_header_list_size = Some(value),
                0x8 => settings.enable_connect_protocol = Some(value != 0),
                0x9 => settings.no_rfc7540_priorities = Some(value != 0),
                _ => {
                    // Unknown settings are ignored per RFC 7540
                }
            }

            pos += 6;
        }

        // Apply settings
        settings.validate()?;
        self.remote_settings.merge(&settings);

        // Update stream manager max concurrent streams
        self.stream_manager
            .set_max_concurrent_streams(settings.max_concurrent_streams);

        // Update flow control if initial window size changed
        if let Some(new_size) = settings.initial_window_size {
            // Update all existing streams
            for stream_id in self.stream_manager.stream_ids() {
                if let Some(stream) = self.stream_manager.get_stream_mut(stream_id) {
                    stream
                        .flow_control_mut()
                        .send_window_mut()
                        .update_initial_size(new_size)?;
                }
            }
        }

        // Send SETTINGS ACK
        self.send_settings_ack()?;

        Ok(())
    }

    /// Send a simple GET request
    pub fn get(&mut self, path: &str) -> Result<H2Response> {
        self.request("GET", path, &[], Bytes::new())
    }

    /// Send a POST request
    pub fn post(&mut self, path: &str, headers: &[(&str, &str)], body: Bytes) -> Result<H2Response> {
        self.request("POST", path, headers, body)
    }

    /// Send an HTTP/2 request
    pub fn request(
        &mut self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: Bytes,
    ) -> Result<H2Response> {
        // Ensure connection is established
        if !self.connected {
            self.connect()?;
        }

        // Create new stream
        let stream_id = self.stream_manager.create_stream()?;

        // Build headers
        let mut hpack_headers = Vec::new();
        hpack_headers.push((":method", method));
        hpack_headers.push((":path", path));
        hpack_headers.push((":scheme", "https"));
        hpack_headers.push((":authority", "localhost"));

        // Add custom headers
        for (name, value) in headers {
            hpack_headers.push((name, value));
        }

        // Encode headers with HPACK
        let mut header_block_vec = Vec::new();
        let header_tuples: Vec<(&[u8], &[u8])> = hpack_headers
            .iter()
            .map(|(name, value)| (name.as_bytes(), value.as_bytes()))
            .collect();
        self.hpack_encoder
            .encode_into(header_tuples, &mut header_block_vec)
            .map_err(|e| Error::Internal(format!("HPACK encode error: {}", e)))?;

        // Send HEADERS frame
        let has_body = !body.is_empty();
        let headers_frame = HeadersFrame::new(
            stream_id,
            Bytes::from(header_block_vec),
            !has_body, // END_STREAM if no body
            true,      // END_HEADERS (no continuation for now)
        );
        self.send_headers(&headers_frame)?;

        // Send DATA frame if there's a body
        if has_body {
            let data_frame = DataFrame::new(stream_id, body, true);
            self.send_data(&data_frame)?;
        }

        // Receive response
        self.recv_response(stream_id)
    }

    /// Send a HEADERS frame
    pub fn send_headers(&mut self, frame: &HeadersFrame) -> Result<()> {
        // Update stream state
        if let Some(stream) = self.stream_manager.get_stream_mut(frame.stream_id) {
            stream.send_headers(frame.end_stream)?;
        }

        let encoded = FrameCodec::encode_headers_frame(frame);
        self.session.write(&encoded)?;
        Ok(())
    }

    /// Send a DATA frame
    pub fn send_data(&mut self, frame: &DataFrame) -> Result<()> {
        // Check connection-level flow control
        let sendable_conn = self.flow_control.consume_send_window(frame.data.len())?;
        if sendable_conn == 0 {
            return Err(Error::FlowControl("Connection window exhausted".to_string()));
        }

        // Check stream-level flow control
        if let Some(stream) = self.stream_manager.get_stream_mut(frame.stream_id) {
            let sendable_stream = stream.send_data(frame.data.len(), frame.end_stream)?;
            if sendable_stream == 0 {
                return Err(Error::FlowControl("Stream window exhausted".to_string()));
            }
        }

        let encoded = FrameCodec::encode_data_frame(frame);
        self.session.write(&encoded)?;
        Ok(())
    }

    /// Send a PING frame
    pub fn send_ping(&mut self, data: [u8; 8]) -> Result<()> {
        let frame = PingFrame::new(data);
        let encoded = FrameCodec::encode_ping_frame(&frame);
        self.session.write(&encoded)?;
        Ok(())
    }

    /// Send a WINDOW_UPDATE frame
    pub fn send_window_update(&mut self, stream_id: StreamId, increment: u32) -> Result<()> {
        let frame = WindowUpdateFrame::new(stream_id, increment);
        let encoded = FrameCodec::encode_window_update_frame(&frame);
        self.session.write(&encoded)?;

        // Update flow control
        if stream_id == CONNECTION_STREAM_ID {
            self.flow_control.increase_send_window(increment)?;
        } else if let Some(stream) = self.stream_manager.get_stream_mut(stream_id) {
            stream.flow_control_mut().increase_send_window(increment)?;
        }

        Ok(())
    }

    /// Send a RST_STREAM frame
    pub fn send_rst_stream(&mut self, stream_id: StreamId, error_code: ErrorCode) -> Result<()> {
        let frame = RstStreamFrame { stream_id, error_code };
        let encoded = FrameCodec::encode_rst_stream_frame(&frame);
        self.session.write(&encoded)?;

        // Close the stream
        if let Some(stream) = self.stream_manager.get_stream_mut(stream_id) {
            stream.close();
        }

        Ok(())
    }

    /// Send a GOAWAY frame
    pub fn send_goaway(&mut self, last_stream_id: StreamId, error_code: ErrorCode, debug: &str) -> Result<()> {
        let frame = GoawayFrame::new(last_stream_id, error_code, Bytes::from(debug.to_string()));
        let encoded = FrameCodec::encode_goaway_frame(&frame);
        self.session.write(&encoded)?;
        Ok(())
    }

    /// Receive a frame
    pub fn recv_frame(&mut self) -> Result<(FrameType, FrameFlags, StreamId, Bytes)> {
        FrameCodec::read_frame_from_session(&mut self.session)
            .map_err(|e| Error::Io(e))
    }

    /// Receive a response for a stream
    pub fn recv_response(&mut self, stream_id: StreamId) -> Result<H2Response> {
        let mut response = H2Response {
            stream_id,
            status: 0,
            headers: HashMap::new(),
            body: Bytes::new(),
        };

        let mut headers_received = false;
        let mut stream_ended = false;

        while !stream_ended {
            let (frame_type, flags, recv_stream_id, payload) = self.recv_frame()?;

            // Skip frames for other streams (interleaved)
            if recv_stream_id != stream_id && recv_stream_id != CONNECTION_STREAM_ID {
                continue;
            }

            match frame_type {
                FrameType::Headers => {
                    if headers_received {
                        // Trailers - ignore for now
                        continue;
                    }

                    // Decode headers with HPACK
                    let decoded = self.hpack_decoder
                        .decode(&payload)
                        .map_err(|e| Error::Compression(format!("HPACK decode error: {:?}", e)))?;

                    for (name, value) in decoded {
                        let name_str = String::from_utf8_lossy(&name).to_string();
                        let value_str = String::from_utf8_lossy(&value).to_string();

                        if name_str == ":status" {
                            response.status = value_str.parse().unwrap_or(0);
                        } else {
                            response.headers.insert(name_str, value_str);
                        }
                    }

                    headers_received = true;

                    if flags.is_end_stream() {
                        stream_ended = true;
                    }
                }
                FrameType::Data => {
                    // Update flow control
                    self.flow_control.consume_recv_window(payload.len());
                    if let Some(stream) = self.stream_manager.get_stream_mut(stream_id) {
                        stream.flow_control_mut().consume_recv_window(payload.len());
                    }

                    response.body = payload;

                    if flags.is_end_stream() {
                        stream_ended = true;
                    }
                }
                FrameType::Settings => {
                    // Handle SETTINGS during response
                    self.recv_settings()?;
                }
                FrameType::WindowUpdate => {
                    // Handle WINDOW_UPDATE
                    if payload.len() != 4 {
                        return Err(Error::FrameSize("WINDOW_UPDATE must be 4 bytes".to_string()));
                    }
                    let increment = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);

                    if recv_stream_id == CONNECTION_STREAM_ID {
                        self.flow_control.increase_send_window(increment)?;
                    } else if let Some(stream) = self.stream_manager.get_stream_mut(recv_stream_id) {
                        stream.flow_control_mut().increase_send_window(increment)?;
                    }
                }
                FrameType::Ping => {
                    // Respond to PING
                    if flags.is_ack() {
                        // PING ACK - ignore
                    } else {
                        // Send PING ACK
                        let mut data = [0u8; 8];
                        data.copy_from_slice(&payload[..8]);
                        let pong = PingFrame::ack(data);
                        let encoded = FrameCodec::encode_ping_frame(&pong);
                        self.session.write(&encoded)?;
                    }
                }
                FrameType::Goaway => {
                    return Err(Error::ConnectionClosed);
                }
                FrameType::RstStream => {
                    // Stream reset
                    if let Some(stream) = self.stream_manager.get_stream_mut(recv_stream_id) {
                        stream.close();
                    }
                    return Err(Error::Cancel(recv_stream_id));
                }
                _ => {
                    // Ignore other frame types
                }
            }
        }

        Ok(response)
    }

    /// Get local settings
    pub fn local_settings(&self) -> &Settings {
        &self.local_settings
    }

    /// Get remote settings
    pub fn remote_settings(&self) -> &Settings {
        &self.remote_settings
    }
}

/// HTTP/2 response
#[derive(Debug, Clone)]
pub struct H2Response {
    /// Stream ID
    pub stream_id: StreamId,
    /// Status code
    pub status: u16,
    /// Headers
    pub headers: HashMap<String, String>,
    /// Body
    pub body: Bytes,
}

impl H2Response {
    /// Get status code
    pub fn status(&self) -> u16 {
        self.status
    }

    /// Get header value
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(|s| s.as_str())
    }

    /// Get body as bytes
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Get body as string
    pub fn body_string(&self) -> Result<String> {
        String::from_utf8(self.body.to_vec())
            .map_err(|e| Error::Internal(format!("Invalid UTF-8 in body: {}", e)))
    }
}

/// HTTP/2 client builder
pub struct H2ClientBuilder {
    settings: SettingsBuilder,
}

impl H2ClientBuilder {
    /// Create a new client builder
    pub fn new() -> Self {
        H2ClientBuilder {
            settings: SettingsBuilder::new()
                .header_table_size(4096)
                .enable_push(false)
                .initial_window_size(65535)
                .max_frame_size(16384),
        }
    }

    /// Set header table size
    pub fn header_table_size(mut self, size: u32) -> Self {
        self.settings = self.settings.header_table_size(size);
        self
    }

    /// Set enable push
    pub fn enable_push(mut self, enable: bool) -> Self {
        self.settings = self.settings.enable_push(enable);
        self
    }

    /// Set initial window size
    pub fn initial_window_size(mut self, size: u32) -> Self {
        self.settings = self.settings.initial_window_size(size);
        self
    }

    /// Set max frame size
    pub fn max_frame_size(mut self, size: u32) -> Self {
        self.settings = self.settings.max_frame_size(size);
        self
    }

    /// Set max concurrent streams
    pub fn max_concurrent_streams(mut self, max: u32) -> Self {
        self.settings = self.settings.max_concurrent_streams(max);
        self
    }

    /// Build the client
    pub fn build<S: SessionOps>(self, session: S) -> Result<H2Client<S>> {
        let local_settings = self.settings.build()?;

        Ok(H2Client {
            session: HttpSession::new(session),
            stream_manager: StreamManager::new(true), // Client uses odd stream IDs
            flow_control: ConnectionFlowControl::new(),
            hpack_encoder: HpackEncoder::new(),
            hpack_decoder: hpack::Decoder::new(),
            local_settings,
            remote_settings: Settings::default_settings(),
            connected: false,
        })
    }
}

impl Default for H2ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::session::FdSessionOps;
    use std::net::TcpStream;

    #[test]
    fn test_client_builder() {
        let builder = H2ClientBuilder::new()
            .header_table_size(8192)
            .enable_push(false)
            .initial_window_size(65535);

        // We can't actually build without a real connection, but we can test the builder API
    }

    #[test]
    fn test_response_accessors() {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "text/plain".to_string());

        let response = H2Response {
            stream_id: 1,
            status: 200,
            headers,
            body: Bytes::from("Hello"),
        };

        assert_eq!(response.status(), 200);
        assert_eq!(response.header("content-type"), Some("text/plain"));
        assert_eq!(response.body(), b"Hello");
        assert_eq!(response.body_string().unwrap(), "Hello");
    }
}
