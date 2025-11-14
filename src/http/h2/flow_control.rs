//! HTTP/2 flow control
//!
//! This module implements flow control as defined in RFC 7540 Section 5.2.
//!
//! HTTP/2 uses flow control to prevent senders from overwhelming receivers
//! with data. Flow control is applied at both the connection and stream level.

use super::error::{Error, Result};
use super::DEFAULT_INITIAL_WINDOW_SIZE;

/// Flow control window
///
/// Tracks the available window size for sending data.
#[derive(Debug, Clone)]
pub struct FlowControlWindow {
    /// Initial window size
    initial_size: u32,
    /// Current window size (can be negative if over-committed)
    current_size: i64,
    /// Maximum window size allowed (2^31 - 1)
    max_size: i64,
}

impl FlowControlWindow {
    /// Create a new flow control window with default size
    pub fn new() -> Self {
        Self::with_initial_size(DEFAULT_INITIAL_WINDOW_SIZE)
    }

    /// Create a new flow control window with specified initial size
    pub fn with_initial_size(initial_size: u32) -> Self {
        FlowControlWindow {
            initial_size,
            current_size: initial_size as i64,
            max_size: 0x7FFFFFFF, // 2^31 - 1
        }
    }

    /// Get current window size
    pub fn size(&self) -> i64 {
        self.current_size
    }

    /// Get initial window size
    pub fn initial_size(&self) -> u32 {
        self.initial_size
    }

    /// Check if window has available capacity
    pub fn has_capacity(&self) -> bool {
        self.current_size > 0
    }

    /// Check if window can send specified amount
    pub fn can_send(&self, amount: usize) -> bool {
        self.current_size >= amount as i64
    }

    /// Consume window capacity for sending data
    ///
    /// Returns the actual amount that can be sent (may be less than requested)
    pub fn consume(&mut self, amount: usize) -> Result<usize> {
        if amount == 0 {
            return Ok(0);
        }

        if self.current_size <= 0 {
            return Ok(0); // No capacity available
        }

        let to_send = std::cmp::min(amount as i64, self.current_size) as usize;
        self.current_size -= to_send as i64;

        Ok(to_send)
    }

    /// Increase window size (WINDOW_UPDATE)
    ///
    /// Returns the new window size
    pub fn increase(&mut self, increment: u32) -> Result<i64> {
        if increment == 0 {
            return Err(Error::FlowControl(
                "Window update increment must be non-zero".to_string(),
            ));
        }

        let new_size = self.current_size + increment as i64;

        // Check for overflow (RFC 7540 Section 6.9.1)
        if new_size > self.max_size {
            return Err(Error::FlowControl(format!(
                "Window size {} exceeds maximum (2^31-1)",
                new_size
            )));
        }

        self.current_size = new_size;
        Ok(self.current_size)
    }

    /// Decrease window size (receiving data)
    pub fn decrease(&mut self, amount: usize) {
        self.current_size -= amount as i64;
    }

    /// Update initial window size from SETTINGS
    ///
    /// This adjusts the current window size proportionally
    pub fn update_initial_size(&mut self, new_initial_size: u32) -> Result<()> {
        // Calculate the difference
        let diff = new_initial_size as i64 - self.initial_size as i64;

        // Apply the difference to current size
        let new_current = self.current_size + diff;

        // Check for overflow
        if new_current > self.max_size {
            return Err(Error::FlowControl(format!(
                "New window size {} exceeds maximum (2^31-1)",
                new_current
            )));
        }

        self.initial_size = new_initial_size;
        self.current_size = new_current;

        Ok(())
    }

    /// Reset window to initial size
    pub fn reset(&mut self) {
        self.current_size = self.initial_size as i64;
    }
}

impl Default for FlowControlWindow {
    fn default() -> Self {
        Self::new()
    }
}

/// Connection-level flow control
///
/// Manages flow control for the entire connection
#[derive(Debug)]
pub struct ConnectionFlowControl {
    /// Send window (outbound data)
    send_window: FlowControlWindow,
    /// Receive window (inbound data)
    recv_window: FlowControlWindow,
}

impl ConnectionFlowControl {
    /// Create new connection-level flow control
    pub fn new() -> Self {
        ConnectionFlowControl {
            send_window: FlowControlWindow::new(),
            recv_window: FlowControlWindow::new(),
        }
    }

    /// Create with specified initial window sizes
    pub fn with_initial_sizes(send_size: u32, recv_size: u32) -> Self {
        ConnectionFlowControl {
            send_window: FlowControlWindow::with_initial_size(send_size),
            recv_window: FlowControlWindow::with_initial_size(recv_size),
        }
    }

    /// Get send window
    pub fn send_window(&self) -> &FlowControlWindow {
        &self.send_window
    }

    /// Get mutable send window
    pub fn send_window_mut(&mut self) -> &mut FlowControlWindow {
        &mut self.send_window
    }

    /// Get receive window
    pub fn recv_window(&self) -> &FlowControlWindow {
        &self.recv_window
    }

    /// Get mutable receive window
    pub fn recv_window_mut(&mut self) -> &mut FlowControlWindow {
        &mut self.recv_window
    }

    /// Check if we can send data on the connection
    pub fn can_send(&self, amount: usize) -> bool {
        self.send_window.can_send(amount)
    }

    /// Consume send window for outbound data
    pub fn consume_send_window(&mut self, amount: usize) -> Result<usize> {
        self.send_window.consume(amount)
    }

    /// Increase send window from WINDOW_UPDATE
    pub fn increase_send_window(&mut self, increment: u32) -> Result<i64> {
        self.send_window.increase(increment)
    }

    /// Decrease receive window for inbound data
    pub fn consume_recv_window(&mut self, amount: usize) {
        self.recv_window.decrease(amount);
    }

    /// Check if we need to send WINDOW_UPDATE
    ///
    /// Returns the suggested increment if an update is needed
    pub fn should_send_window_update(&self) -> Option<u32> {
        let recv_size = self.recv_window.size();
        let initial_size = self.recv_window.initial_size() as i64;

        // Send update if window is less than half of initial size
        if recv_size < initial_size / 2 {
            let increment = (initial_size - recv_size) as u32;
            Some(increment)
        } else {
            None
        }
    }

    /// Send WINDOW_UPDATE (increases receive window)
    pub fn send_window_update(&mut self, increment: u32) -> Result<i64> {
        self.recv_window.increase(increment)
    }
}

impl Default for ConnectionFlowControl {
    fn default() -> Self {
        Self::new()
    }
}

/// Stream-level flow control
///
/// Manages flow control for an individual stream
#[derive(Debug)]
pub struct StreamFlowControl {
    /// Stream ID
    stream_id: u32,
    /// Send window (outbound data)
    send_window: FlowControlWindow,
    /// Receive window (inbound data)
    recv_window: FlowControlWindow,
}

impl StreamFlowControl {
    /// Create new stream-level flow control
    pub fn new(stream_id: u32) -> Self {
        StreamFlowControl {
            stream_id,
            send_window: FlowControlWindow::new(),
            recv_window: FlowControlWindow::new(),
        }
    }

    /// Create with specified initial window sizes
    pub fn with_initial_sizes(stream_id: u32, send_size: u32, recv_size: u32) -> Self {
        StreamFlowControl {
            stream_id,
            send_window: FlowControlWindow::with_initial_size(send_size),
            recv_window: FlowControlWindow::with_initial_size(recv_size),
        }
    }

    /// Get stream ID
    pub fn stream_id(&self) -> u32 {
        self.stream_id
    }

    /// Get send window
    pub fn send_window(&self) -> &FlowControlWindow {
        &self.send_window
    }

    /// Get mutable send window
    pub fn send_window_mut(&mut self) -> &mut FlowControlWindow {
        &mut self.send_window
    }

    /// Get receive window
    pub fn recv_window(&self) -> &FlowControlWindow {
        &self.recv_window
    }

    /// Get mutable receive window
    pub fn recv_window_mut(&mut self) -> &mut FlowControlWindow {
        &mut self.recv_window
    }

    /// Check if we can send data on this stream
    pub fn can_send(&self, amount: usize) -> bool {
        self.send_window.can_send(amount)
    }

    /// Consume send window for outbound data
    pub fn consume_send_window(&mut self, amount: usize) -> Result<usize> {
        self.send_window.consume(amount)
    }

    /// Increase send window from WINDOW_UPDATE
    pub fn increase_send_window(&mut self, increment: u32) -> Result<i64> {
        self.send_window.increase(increment)
    }

    /// Decrease receive window for inbound data
    pub fn consume_recv_window(&mut self, amount: usize) {
        self.recv_window.decrease(amount);
    }

    /// Check if we need to send WINDOW_UPDATE
    pub fn should_send_window_update(&self) -> Option<u32> {
        let recv_size = self.recv_window.size();
        let initial_size = self.recv_window.initial_size() as i64;

        // Send update if window is less than half of initial size
        if recv_size < initial_size / 2 {
            let increment = (initial_size - recv_size) as u32;
            Some(increment)
        } else {
            None
        }
    }

    /// Send WINDOW_UPDATE (increases receive window)
    pub fn send_window_update(&mut self, increment: u32) -> Result<i64> {
        self.recv_window.increase(increment)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flow_control_window_basic() {
        let mut window = FlowControlWindow::new();
        assert_eq!(window.size(), DEFAULT_INITIAL_WINDOW_SIZE as i64);
        assert!(window.has_capacity());
    }

    #[test]
    fn test_flow_control_window_consume() {
        let mut window = FlowControlWindow::with_initial_size(100);
        assert_eq!(window.size(), 100);

        let consumed = window.consume(50).unwrap();
        assert_eq!(consumed, 50);
        assert_eq!(window.size(), 50);

        let consumed = window.consume(60).unwrap();
        assert_eq!(consumed, 50); // Only 50 available
        assert_eq!(window.size(), 0);

        let consumed = window.consume(10).unwrap();
        assert_eq!(consumed, 0); // No capacity
    }

    #[test]
    fn test_flow_control_window_increase() {
        let mut window = FlowControlWindow::with_initial_size(100);
        window.consume(50).unwrap();
        assert_eq!(window.size(), 50);

        window.increase(100).unwrap();
        assert_eq!(window.size(), 150);
    }

    #[test]
    fn test_flow_control_window_overflow() {
        let mut window = FlowControlWindow::with_initial_size(0x7FFFFFFF);
        let result = window.increase(1);
        assert!(result.is_err());
    }

    #[test]
    fn test_flow_control_window_update_initial_size() {
        let mut window = FlowControlWindow::with_initial_size(100);
        window.consume(50).unwrap();
        assert_eq!(window.size(), 50);

        // Increase initial size by 100
        window.update_initial_size(200).unwrap();
        assert_eq!(window.initial_size(), 200);
        assert_eq!(window.size(), 150); // 50 + 100

        // Decrease initial size by 50
        window.update_initial_size(150).unwrap();
        assert_eq!(window.initial_size(), 150);
        assert_eq!(window.size(), 100); // 150 - 50
    }

    #[test]
    fn test_connection_flow_control() {
        let mut flow_control = ConnectionFlowControl::new();

        // Test send window
        assert!(flow_control.can_send(1000));
        let consumed = flow_control.consume_send_window(1000).unwrap();
        assert_eq!(consumed, 1000);

        // Test receive window
        flow_control.consume_recv_window(1000);
        assert_eq!(
            flow_control.recv_window().size(),
            (DEFAULT_INITIAL_WINDOW_SIZE - 1000) as i64
        );

        // Test window update
        flow_control.increase_send_window(500).unwrap();
        assert!(flow_control.can_send(500));
    }

    #[test]
    fn test_stream_flow_control() {
        let mut flow_control = StreamFlowControl::new(42);
        assert_eq!(flow_control.stream_id(), 42);

        // Test send window
        assert!(flow_control.can_send(1000));
        let consumed = flow_control.consume_send_window(1000).unwrap();
        assert_eq!(consumed, 1000);

        // Test receive window
        flow_control.consume_recv_window(1000);
        assert_eq!(
            flow_control.recv_window().size(),
            (DEFAULT_INITIAL_WINDOW_SIZE - 1000) as i64
        );
    }

    #[test]
    fn test_should_send_window_update() {
        let mut flow_control = ConnectionFlowControl::with_initial_sizes(100, 100);

        // No update needed initially
        assert_eq!(flow_control.should_send_window_update(), None);

        // Consume more than half
        flow_control.consume_recv_window(60);
        let update = flow_control.should_send_window_update();
        assert!(update.is_some());
        assert_eq!(update.unwrap(), 60);
    }
}
