//! TLS handshake logic
//!
//! This module provides utilities for TLS handshake operations.
//! Note: The actual handshake is performed by the `openssl` crate's
//! `Ssl::connect()` and `Ssl::accept()` methods.

use super::config::TlsError;

/// Default handshake timeout (used by session module)
pub const DEFAULT_HANDSHAKE_TIMEOUT_SECS: u64 = 10;

/// Handshake result helper
pub type HandshakeResult = std::result::Result<(), TlsError>;

// Note: The handshake logic has been moved to the session module
// where it's implemented using Ssl::connect() and Ssl::accept()
// which handle the complexity of the TLS handshake internally.
//
// This module is kept for future enhancements like custom
// handshake timeouts, retry logic, or handshake callbacks.

#[cfg(test)]
mod tests {
    #[test]
    fn test_handshake_timeout_constant() {
        assert_eq!(super::DEFAULT_HANDSHAKE_TIMEOUT_SECS, 10);
    }
}
