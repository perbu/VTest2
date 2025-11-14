//! HTTP/2 settings management
//!
//! This module implements HTTP/2 SETTINGS frames and parameters
//! as defined in RFC 7540 Section 6.5.

use super::error::{Error, Result};
use std::fmt;

/// HTTP/2 settings parameters (RFC 7540 Section 6.5.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum SettingsParameter {
    /// SETTINGS_HEADER_TABLE_SIZE (0x1)
    /// Allows the sender to inform the remote endpoint of the maximum size
    /// of the header compression table
    HeaderTableSize = 0x1,

    /// SETTINGS_ENABLE_PUSH (0x2)
    /// Used to disable server push
    EnablePush = 0x2,

    /// SETTINGS_MAX_CONCURRENT_STREAMS (0x3)
    /// Indicates the maximum number of concurrent streams
    MaxConcurrentStreams = 0x3,

    /// SETTINGS_INITIAL_WINDOW_SIZE (0x4)
    /// Indicates the sender's initial window size for stream-level flow control
    InitialWindowSize = 0x4,

    /// SETTINGS_MAX_FRAME_SIZE (0x5)
    /// Indicates the size of the largest frame payload
    MaxFrameSize = 0x5,

    /// SETTINGS_MAX_HEADER_LIST_SIZE (0x6)
    /// Advises peer of the maximum size of header list
    MaxHeaderListSize = 0x6,

    /// SETTINGS_ENABLE_CONNECT_PROTOCOL (0x8) - RFC 8441
    /// Enables support for CONNECT requests with the :protocol pseudo-header
    EnableConnectProtocol = 0x8,

    /// SETTINGS_NO_RFC7540_PRIORITIES (0x9) - RFC 9218
    /// Indicates that HTTP/2 priorities defined in RFC 7540 are not supported
    NoRfc7540Priorities = 0x9,
}

impl SettingsParameter {
    /// Convert to u16
    pub fn as_u16(self) -> u16 {
        self as u16
    }

    /// Create from u16
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            0x1 => Some(SettingsParameter::HeaderTableSize),
            0x2 => Some(SettingsParameter::EnablePush),
            0x3 => Some(SettingsParameter::MaxConcurrentStreams),
            0x4 => Some(SettingsParameter::InitialWindowSize),
            0x5 => Some(SettingsParameter::MaxFrameSize),
            0x6 => Some(SettingsParameter::MaxHeaderListSize),
            0x8 => Some(SettingsParameter::EnableConnectProtocol),
            0x9 => Some(SettingsParameter::NoRfc7540Priorities),
            _ => None,
        }
    }

    /// Get parameter name
    pub fn name(&self) -> &'static str {
        match self {
            SettingsParameter::HeaderTableSize => "HEADER_TABLE_SIZE",
            SettingsParameter::EnablePush => "ENABLE_PUSH",
            SettingsParameter::MaxConcurrentStreams => "MAX_CONCURRENT_STREAMS",
            SettingsParameter::InitialWindowSize => "INITIAL_WINDOW_SIZE",
            SettingsParameter::MaxFrameSize => "MAX_FRAME_SIZE",
            SettingsParameter::MaxHeaderListSize => "MAX_HEADER_LIST_SIZE",
            SettingsParameter::EnableConnectProtocol => "ENABLE_CONNECT_PROTOCOL",
            SettingsParameter::NoRfc7540Priorities => "NO_RFC7540_PRIORITIES",
        }
    }
}

impl fmt::Display for SettingsParameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (0x{:x})", self.name(), self.as_u16())
    }
}

/// HTTP/2 settings
#[derive(Debug, Clone)]
pub struct Settings {
    /// Header table size (default: 4096)
    pub header_table_size: Option<u32>,

    /// Enable server push (default: true)
    pub enable_push: Option<bool>,

    /// Maximum concurrent streams (default: unlimited)
    pub max_concurrent_streams: Option<u32>,

    /// Initial window size (default: 65535)
    pub initial_window_size: Option<u32>,

    /// Maximum frame size (default: 16384, range: 16384-16777215)
    pub max_frame_size: Option<u32>,

    /// Maximum header list size (default: unlimited)
    pub max_header_list_size: Option<u32>,

    /// Enable CONNECT protocol (default: false)
    pub enable_connect_protocol: Option<bool>,

    /// Disable RFC 7540 priorities (default: false)
    pub no_rfc7540_priorities: Option<bool>,
}

impl Settings {
    /// Create empty settings
    pub fn new() -> Self {
        Settings {
            header_table_size: None,
            enable_push: None,
            max_concurrent_streams: None,
            initial_window_size: None,
            max_frame_size: None,
            max_header_list_size: None,
            enable_connect_protocol: None,
            no_rfc7540_priorities: None,
        }
    }

    /// Create default settings
    pub fn default_settings() -> Self {
        Settings {
            header_table_size: Some(4096),
            enable_push: Some(true),
            max_concurrent_streams: None, // Unlimited
            initial_window_size: Some(65535),
            max_frame_size: Some(16384),
            max_header_list_size: None, // Unlimited
            enable_connect_protocol: Some(false),
            no_rfc7540_priorities: Some(false),
        }
    }

    /// Get header table size (with default)
    pub fn get_header_table_size(&self) -> u32 {
        self.header_table_size.unwrap_or(4096)
    }

    /// Get enable push (with default)
    pub fn get_enable_push(&self) -> bool {
        self.enable_push.unwrap_or(true)
    }

    /// Get max concurrent streams (None = unlimited)
    pub fn get_max_concurrent_streams(&self) -> Option<u32> {
        self.max_concurrent_streams
    }

    /// Get initial window size (with default)
    pub fn get_initial_window_size(&self) -> u32 {
        self.initial_window_size.unwrap_or(65535)
    }

    /// Get max frame size (with default)
    pub fn get_max_frame_size(&self) -> u32 {
        self.max_frame_size.unwrap_or(16384)
    }

    /// Get max header list size (None = unlimited)
    pub fn get_max_header_list_size(&self) -> Option<u32> {
        self.max_header_list_size
    }

    /// Get enable CONNECT protocol (with default)
    pub fn get_enable_connect_protocol(&self) -> bool {
        self.enable_connect_protocol.unwrap_or(false)
    }

    /// Get no RFC 7540 priorities (with default)
    pub fn get_no_rfc7540_priorities(&self) -> bool {
        self.no_rfc7540_priorities.unwrap_or(false)
    }

    /// Validate settings values
    pub fn validate(&self) -> Result<()> {
        // Validate SETTINGS_ENABLE_PUSH (must be 0 or 1)
        if let Some(enable_push) = self.enable_push {
            // Already a bool, so always valid
        }

        // Validate SETTINGS_INITIAL_WINDOW_SIZE (max 2^31-1)
        if let Some(initial_window_size) = self.initial_window_size {
            if initial_window_size > 0x7FFFFFFF {
                return Err(Error::InvalidSettings(format!(
                    "Initial window size {} exceeds maximum (2^31-1)",
                    initial_window_size
                )));
            }
        }

        // Validate SETTINGS_MAX_FRAME_SIZE (16384 to 16777215)
        if let Some(max_frame_size) = self.max_frame_size {
            if max_frame_size < 16384 || max_frame_size > 16777215 {
                return Err(Error::InvalidSettings(format!(
                    "Max frame size {} outside valid range (16384-16777215)",
                    max_frame_size
                )));
            }
        }

        Ok(())
    }

    /// Merge settings from another Settings object
    /// (values in `other` override values in `self`)
    pub fn merge(&mut self, other: &Settings) {
        if other.header_table_size.is_some() {
            self.header_table_size = other.header_table_size;
        }
        if other.enable_push.is_some() {
            self.enable_push = other.enable_push;
        }
        if other.max_concurrent_streams.is_some() {
            self.max_concurrent_streams = other.max_concurrent_streams;
        }
        if other.initial_window_size.is_some() {
            self.initial_window_size = other.initial_window_size;
        }
        if other.max_frame_size.is_some() {
            self.max_frame_size = other.max_frame_size;
        }
        if other.max_header_list_size.is_some() {
            self.max_header_list_size = other.max_header_list_size;
        }
        if other.enable_connect_protocol.is_some() {
            self.enable_connect_protocol = other.enable_connect_protocol;
        }
        if other.no_rfc7540_priorities.is_some() {
            self.no_rfc7540_priorities = other.no_rfc7540_priorities;
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings::new()
    }
}

/// Builder for HTTP/2 settings
pub struct SettingsBuilder {
    settings: Settings,
}

impl SettingsBuilder {
    /// Create a new settings builder
    pub fn new() -> Self {
        SettingsBuilder {
            settings: Settings::new(),
        }
    }

    /// Set header table size
    pub fn header_table_size(mut self, size: u32) -> Self {
        self.settings.header_table_size = Some(size);
        self
    }

    /// Set enable push
    pub fn enable_push(mut self, enable: bool) -> Self {
        self.settings.enable_push = Some(enable);
        self
    }

    /// Set max concurrent streams
    pub fn max_concurrent_streams(mut self, max: u32) -> Self {
        self.settings.max_concurrent_streams = Some(max);
        self
    }

    /// Set initial window size
    pub fn initial_window_size(mut self, size: u32) -> Self {
        self.settings.initial_window_size = Some(size);
        self
    }

    /// Set max frame size
    pub fn max_frame_size(mut self, size: u32) -> Self {
        self.settings.max_frame_size = Some(size);
        self
    }

    /// Set max header list size
    pub fn max_header_list_size(mut self, size: u32) -> Self {
        self.settings.max_header_list_size = Some(size);
        self
    }

    /// Set enable CONNECT protocol
    pub fn enable_connect_protocol(mut self, enable: bool) -> Self {
        self.settings.enable_connect_protocol = Some(enable);
        self
    }

    /// Set no RFC 7540 priorities
    pub fn no_rfc7540_priorities(mut self, disable: bool) -> Self {
        self.settings.no_rfc7540_priorities = Some(disable);
        self
    }

    /// Build the settings
    pub fn build(self) -> Result<Settings> {
        self.settings.validate()?;
        Ok(self.settings)
    }
}

impl Default for SettingsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_parameter_conversion() {
        assert_eq!(SettingsParameter::HeaderTableSize.as_u16(), 0x1);
        assert_eq!(SettingsParameter::EnablePush.as_u16(), 0x2);

        assert_eq!(
            SettingsParameter::from_u16(0x1),
            Some(SettingsParameter::HeaderTableSize)
        );
        assert_eq!(
            SettingsParameter::from_u16(0x2),
            Some(SettingsParameter::EnablePush)
        );
        assert_eq!(SettingsParameter::from_u16(0xff), None);
    }

    #[test]
    fn test_settings_defaults() {
        let settings = Settings::default_settings();
        assert_eq!(settings.get_header_table_size(), 4096);
        assert_eq!(settings.get_enable_push(), true);
        assert_eq!(settings.get_initial_window_size(), 65535);
        assert_eq!(settings.get_max_frame_size(), 16384);
    }

    #[test]
    fn test_settings_builder() {
        let settings = SettingsBuilder::new()
            .header_table_size(8192)
            .enable_push(false)
            .max_concurrent_streams(100)
            .initial_window_size(65535)
            .build()
            .unwrap();

        assert_eq!(settings.get_header_table_size(), 8192);
        assert_eq!(settings.get_enable_push(), false);
        assert_eq!(settings.get_max_concurrent_streams(), Some(100));
        assert_eq!(settings.get_initial_window_size(), 65535);
    }

    #[test]
    fn test_settings_validation() {
        // Valid settings
        let settings = SettingsBuilder::new()
            .initial_window_size(65535)
            .max_frame_size(16384)
            .build();
        assert!(settings.is_ok());

        // Invalid initial window size (too large)
        let settings = SettingsBuilder::new()
            .initial_window_size(0x80000000) // 2^31
            .build();
        assert!(settings.is_err());

        // Invalid max frame size (too small)
        let settings = SettingsBuilder::new()
            .max_frame_size(1024) // < 16384
            .build();
        assert!(settings.is_err());

        // Invalid max frame size (too large)
        let settings = SettingsBuilder::new()
            .max_frame_size(16777216) // > 16777215
            .build();
        assert!(settings.is_err());
    }

    #[test]
    fn test_settings_merge() {
        let mut settings1 = SettingsBuilder::new()
            .header_table_size(4096)
            .enable_push(true)
            .build()
            .unwrap();

        let settings2 = SettingsBuilder::new()
            .header_table_size(8192)
            .max_concurrent_streams(100)
            .build()
            .unwrap();

        settings1.merge(&settings2);

        assert_eq!(settings1.get_header_table_size(), 8192); // Overridden
        assert_eq!(settings1.get_enable_push(), true); // Unchanged
        assert_eq!(settings1.get_max_concurrent_streams(), Some(100)); // Added
    }
}
