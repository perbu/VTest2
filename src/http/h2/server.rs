//! HTTP/2 server implementation (stub)
//!
//! This module will provide HTTP/2 server functionality with low-level frame control.
//! Currently a placeholder for future implementation.

use super::error::{Error, Result};

/// HTTP/2 server (placeholder)
///
/// Full server implementation coming soon.
pub struct H2Server {
    // Placeholder
}

/// HTTP/2 server builder (placeholder)
pub struct H2ServerBuilder {
    // Placeholder
}

impl H2ServerBuilder {
    /// Create a new server builder
    pub fn new() -> Self {
        H2ServerBuilder {}
    }
}

impl Default for H2ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// Note: Full H2Server implementation will follow the same pattern as H2Client
// with low-level frame control for testing purposes.
