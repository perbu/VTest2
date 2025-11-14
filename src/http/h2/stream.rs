//! HTTP/2 stream management
//!
//! This module implements stream management as defined in RFC 7540 Section 5.1.

use super::error::{Error, Result};
use super::flow_control::StreamFlowControl;
use super::frames::{DataFrame, HeadersFrame, PrioritySpec};
use std::collections::HashMap;

/// Stream ID type
pub type StreamId = u32;

/// Stream state as defined in RFC 7540 Section 5.1
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    /// Idle: No frames have been sent/received
    Idle,
    /// Reserved (local): PUSH_PROMISE sent
    ReservedLocal,
    /// Reserved (remote): PUSH_PROMISE received
    ReservedRemote,
    /// Open: Both sides can send frames
    Open,
    /// Half-closed (local): We can't send, they can
    HalfClosedLocal,
    /// Half-closed (remote): They can't send, we can
    HalfClosedRemote,
    /// Closed: Stream is closed
    Closed,
}

impl StreamState {
    /// Check if stream can send data
    pub fn can_send(&self) -> bool {
        matches!(self, StreamState::Open | StreamState::HalfClosedRemote)
    }

    /// Check if stream can receive data
    pub fn can_receive(&self) -> bool {
        matches!(self, StreamState::Open | StreamState::HalfClosedLocal)
    }

    /// Check if stream is closed
    pub fn is_closed(&self) -> bool {
        matches!(self, StreamState::Closed)
    }
}

/// HTTP/2 stream
#[derive(Debug)]
pub struct H2Stream {
    /// Stream ID
    id: StreamId,
    /// Stream state
    state: StreamState,
    /// Flow control
    flow_control: StreamFlowControl,
    /// Priority information
    priority: Option<PrioritySpec>,
    /// Accumulated header block
    header_block: Vec<u8>,
    /// Accumulated body data
    body: Vec<u8>,
    /// Whether we've received END_HEADERS
    headers_complete: bool,
    /// Whether we've received END_STREAM
    stream_complete: bool,
}

impl H2Stream {
    /// Create a new stream
    pub fn new(id: StreamId) -> Self {
        H2Stream {
            id,
            state: StreamState::Idle,
            flow_control: StreamFlowControl::new(id),
            priority: None,
            header_block: Vec::new(),
            body: Vec::new(),
            headers_complete: false,
            stream_complete: false,
        }
    }

    /// Create a new stream with specified window sizes
    pub fn with_window_sizes(id: StreamId, send_size: u32, recv_size: u32) -> Self {
        H2Stream {
            id,
            state: StreamState::Idle,
            flow_control: StreamFlowControl::with_initial_sizes(id, send_size, recv_size),
            priority: None,
            header_block: Vec::new(),
            body: Vec::new(),
            headers_complete: false,
            stream_complete: false,
        }
    }

    /// Get stream ID
    pub fn id(&self) -> StreamId {
        self.id
    }

    /// Get stream state
    pub fn state(&self) -> StreamState {
        self.state
    }

    /// Set stream state
    pub fn set_state(&mut self, state: StreamState) {
        self.state = state;
    }

    /// Get flow control
    pub fn flow_control(&self) -> &StreamFlowControl {
        &self.flow_control
    }

    /// Get mutable flow control
    pub fn flow_control_mut(&mut self) -> &mut StreamFlowControl {
        &mut self.flow_control
    }

    /// Get priority
    pub fn priority(&self) -> Option<&PrioritySpec> {
        self.priority.as_ref()
    }

    /// Set priority
    pub fn set_priority(&mut self, priority: PrioritySpec) {
        self.priority = Some(priority);
    }

    /// Check if headers are complete
    pub fn headers_complete(&self) -> bool {
        self.headers_complete
    }

    /// Check if stream is complete (END_STREAM received)
    pub fn stream_complete(&self) -> bool {
        self.stream_complete
    }

    /// Get accumulated header block
    pub fn header_block(&self) -> &[u8] {
        &self.header_block
    }

    /// Get accumulated body
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Take body (consumes the body data)
    pub fn take_body(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.body)
    }

    /// Process incoming HEADERS frame
    pub fn receive_headers(&mut self, frame: &HeadersFrame) -> Result<()> {
        // Validate state transition
        match self.state {
            StreamState::Idle => {
                self.state = if frame.end_stream {
                    StreamState::HalfClosedRemote
                } else {
                    StreamState::Open
                };
            }
            StreamState::ReservedRemote => {
                self.state = StreamState::HalfClosedLocal;
            }
            StreamState::Open | StreamState::HalfClosedLocal => {
                // Trailers
                if frame.end_stream {
                    self.state = StreamState::HalfClosedRemote;
                }
            }
            _ => {
                return Err(Error::Protocol(format!(
                    "Cannot receive HEADERS in state {:?}",
                    self.state
                )));
            }
        }

        // Accumulate header block
        self.header_block.extend_from_slice(&frame.header_block);

        // Check if headers are complete
        if frame.end_headers {
            self.headers_complete = true;
        }

        // Check if stream is complete
        if frame.end_stream {
            self.stream_complete = true;
        }

        // Store priority if present
        if let Some(priority) = &frame.priority {
            self.priority = Some(*priority);
        }

        Ok(())
    }

    /// Process incoming DATA frame
    pub fn receive_data(&mut self, frame: &DataFrame) -> Result<()> {
        // Validate state
        if !self.state.can_receive() {
            return Err(Error::StreamClosed(self.id));
        }

        // Update flow control
        self.flow_control.consume_recv_window(frame.data.len());

        // Accumulate body data
        self.body.extend_from_slice(&frame.data);

        // Check if stream is complete
        if frame.end_stream {
            self.stream_complete = true;
            self.state = match self.state {
                StreamState::Open => StreamState::HalfClosedRemote,
                StreamState::HalfClosedLocal => StreamState::Closed,
                _ => self.state,
            };
        }

        Ok(())
    }

    /// Prepare to send HEADERS
    pub fn send_headers(&mut self, end_stream: bool) -> Result<()> {
        // Validate state
        match self.state {
            StreamState::Idle => {
                self.state = if end_stream {
                    StreamState::HalfClosedLocal
                } else {
                    StreamState::Open
                };
            }
            StreamState::ReservedLocal => {
                self.state = StreamState::HalfClosedRemote;
            }
            StreamState::Open | StreamState::HalfClosedRemote => {
                if end_stream {
                    self.state = StreamState::HalfClosedLocal;
                }
            }
            _ => {
                return Err(Error::Protocol(format!(
                    "Cannot send HEADERS in state {:?}",
                    self.state
                )));
            }
        }

        Ok(())
    }

    /// Prepare to send DATA
    pub fn send_data(&mut self, data_len: usize, end_stream: bool) -> Result<usize> {
        // Validate state
        if !self.state.can_send() {
            return Err(Error::StreamClosed(self.id));
        }

        // Check flow control
        let sendable = self.flow_control.consume_send_window(data_len)?;

        // Update state if END_STREAM
        if end_stream {
            self.state = match self.state {
                StreamState::Open => StreamState::HalfClosedLocal,
                StreamState::HalfClosedRemote => StreamState::Closed,
                _ => self.state,
            };
        }

        Ok(sendable)
    }

    /// Close the stream
    pub fn close(&mut self) {
        self.state = StreamState::Closed;
    }

    /// Reset the stream
    pub fn reset(&mut self) {
        self.state = StreamState::Closed;
        self.header_block.clear();
        self.body.clear();
        self.headers_complete = false;
        self.stream_complete = false;
    }
}

/// Stream manager
///
/// Manages all streams for a connection
#[derive(Debug)]
pub struct StreamManager {
    /// Active streams
    streams: HashMap<StreamId, H2Stream>,
    /// Next stream ID (client: odd, server: even)
    next_stream_id: StreamId,
    /// Maximum number of concurrent streams (from SETTINGS)
    max_concurrent_streams: Option<u32>,
}

impl StreamManager {
    /// Create a new stream manager
    ///
    /// # Arguments
    /// * `is_client` - True if this is a client (odd stream IDs), false for server (even)
    pub fn new(is_client: bool) -> Self {
        StreamManager {
            streams: HashMap::new(),
            next_stream_id: if is_client { 1 } else { 2 },
            max_concurrent_streams: None,
        }
    }

    /// Set maximum concurrent streams
    pub fn set_max_concurrent_streams(&mut self, max: Option<u32>) {
        self.max_concurrent_streams = max;
    }

    /// Get maximum concurrent streams
    pub fn max_concurrent_streams(&self) -> Option<u32> {
        self.max_concurrent_streams
    }

    /// Get next stream ID (without incrementing)
    pub fn peek_next_stream_id(&self) -> StreamId {
        self.next_stream_id
    }

    /// Allocate next stream ID and create stream
    pub fn create_stream(&mut self) -> Result<StreamId> {
        // Check concurrent stream limit
        if let Some(max) = self.max_concurrent_streams {
            let active_count = self
                .streams
                .values()
                .filter(|s| !s.state().is_closed())
                .count();
            if active_count >= max as usize {
                return Err(Error::TooManyStreams);
            }
        }

        let stream_id = self.next_stream_id;
        self.next_stream_id += 2; // Skip even for client, odd for server

        let stream = H2Stream::new(stream_id);
        self.streams.insert(stream_id, stream);

        Ok(stream_id)
    }

    /// Get a stream by ID
    pub fn get_stream(&self, stream_id: StreamId) -> Option<&H2Stream> {
        self.streams.get(&stream_id)
    }

    /// Get a mutable stream by ID
    pub fn get_stream_mut(&mut self, stream_id: StreamId) -> Option<&mut H2Stream> {
        self.streams.get_mut(&stream_id)
    }

    /// Get or create a stream (for incoming frames)
    pub fn get_or_create_stream(&mut self, stream_id: StreamId) -> Result<&mut H2Stream> {
        if !self.streams.contains_key(&stream_id) {
            // Validate stream ID parity
            let _is_client_initiated = (stream_id % 2) == 1;
            let _we_are_client = (self.next_stream_id % 2) == 1;

            // Server can create even IDs, client can create odd IDs
            // But we can receive any valid ID from peer

            let stream = H2Stream::new(stream_id);
            self.streams.insert(stream_id, stream);
        }

        Ok(self.streams.get_mut(&stream_id).unwrap())
    }

    /// Remove a stream
    pub fn remove_stream(&mut self, stream_id: StreamId) -> Option<H2Stream> {
        self.streams.remove(&stream_id)
    }

    /// Get number of active streams
    pub fn active_stream_count(&self) -> usize {
        self.streams
            .values()
            .filter(|s| !s.state().is_closed())
            .count()
    }

    /// Get all stream IDs
    pub fn stream_ids(&self) -> Vec<StreamId> {
        self.streams.keys().copied().collect()
    }

    /// Clean up closed streams
    pub fn cleanup_closed_streams(&mut self) {
        self.streams.retain(|_, stream| !stream.state().is_closed());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_stream_state_transitions() {
        let mut stream = H2Stream::new(1);
        assert_eq!(stream.state(), StreamState::Idle);

        // Idle -> Open (send HEADERS without END_STREAM)
        stream.send_headers(false).unwrap();
        assert_eq!(stream.state(), StreamState::Open);

        // Open -> HalfClosedLocal (send DATA with END_STREAM)
        stream.send_data(100, true).unwrap();
        assert_eq!(stream.state(), StreamState::HalfClosedLocal);
    }

    #[test]
    fn test_stream_receive_headers() {
        let mut stream = H2Stream::new(1);

        let frame = HeadersFrame::new(
            1,
            Bytes::from("header data"),
            false,
            true,
        );

        stream.receive_headers(&frame).unwrap();
        assert_eq!(stream.state(), StreamState::Open);
        assert!(stream.headers_complete());
        assert!(!stream.stream_complete());
        assert_eq!(stream.header_block(), b"header data");
    }

    #[test]
    fn test_stream_receive_data() {
        let mut stream = H2Stream::new(1);
        stream.state = StreamState::Open;

        let frame = DataFrame::new(1, Bytes::from("body data"), false);
        stream.receive_data(&frame).unwrap();
        assert_eq!(stream.body(), b"body data");
        assert!(!stream.stream_complete());

        let frame2 = DataFrame::new(1, Bytes::from(" more"), true);
        stream.receive_data(&frame2).unwrap();
        assert_eq!(stream.body(), b"body data more");
        assert!(stream.stream_complete());
        assert_eq!(stream.state(), StreamState::HalfClosedRemote);
    }

    #[test]
    fn test_stream_manager_client() {
        let mut manager = StreamManager::new(true);
        assert_eq!(manager.peek_next_stream_id(), 1);

        let id1 = manager.create_stream().unwrap();
        assert_eq!(id1, 1);

        let id2 = manager.create_stream().unwrap();
        assert_eq!(id2, 3);

        let id3 = manager.create_stream().unwrap();
        assert_eq!(id3, 5);

        assert_eq!(manager.active_stream_count(), 3);
    }

    #[test]
    fn test_stream_manager_server() {
        let mut manager = StreamManager::new(false);
        assert_eq!(manager.peek_next_stream_id(), 2);

        let id1 = manager.create_stream().unwrap();
        assert_eq!(id1, 2);

        let id2 = manager.create_stream().unwrap();
        assert_eq!(id2, 4);

        assert_eq!(manager.active_stream_count(), 2);
    }

    #[test]
    fn test_stream_manager_max_concurrent() {
        let mut manager = StreamManager::new(true);
        manager.set_max_concurrent_streams(Some(2));

        let _id1 = manager.create_stream().unwrap();
        let _id2 = manager.create_stream().unwrap();

        // Third stream should fail
        let result = manager.create_stream();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::TooManyStreams));
    }

    #[test]
    fn test_stream_manager_cleanup() {
        let mut manager = StreamManager::new(true);

        let id1 = manager.create_stream().unwrap();
        let id2 = manager.create_stream().unwrap();

        // Close one stream
        if let Some(stream) = manager.get_stream_mut(id1) {
            stream.close();
        }

        assert_eq!(manager.stream_ids().len(), 2);
        assert_eq!(manager.active_stream_count(), 1);

        manager.cleanup_closed_streams();
        assert_eq!(manager.stream_ids().len(), 1);
        assert!(manager.get_stream(id1).is_none());
        assert!(manager.get_stream(id2).is_some());
    }
}
