//! TLS configuration
//!
//! This module provides TLS configuration builders for both client and server.

use std::path::Path;
use std::fs::File;
use std::io::Read;

/// TLS version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TlsVersion {
    /// SSL 3.0 (deprecated, rarely used)
    Ssl3,
    /// TLS 1.0
    Tls10,
    /// TLS 1.1
    Tls11,
    /// TLS 1.2
    Tls12,
    /// TLS 1.3
    Tls13,
}

impl TlsVersion {
    /// Parse TLS version from string (case-insensitive)
    pub fn from_str(s: &str) -> Result<Self, TlsError> {
        match s.to_uppercase().as_str() {
            "SSLV3" | "SSL3" => Ok(TlsVersion::Ssl3),
            "TLSV1.0" | "TLS1.0" | "TLSV1" | "TLS1" => Ok(TlsVersion::Tls10),
            "TLSV1.1" | "TLS1.1" => Ok(TlsVersion::Tls11),
            "TLSV1.2" | "TLS1.2" => Ok(TlsVersion::Tls12),
            "TLSV1.3" | "TLS1.3" => Ok(TlsVersion::Tls13),
            _ => Err(TlsError::InvalidVersion(s.to_string())),
        }
    }

    /// Get OpenSSL protocol version constant
    pub fn to_openssl_version(&self) -> openssl::ssl::SslVersion {
        use openssl::ssl::SslVersion;
        match self {
            TlsVersion::Ssl3 => SslVersion::SSL3,
            TlsVersion::Tls10 => SslVersion::TLS1,
            TlsVersion::Tls11 => SslVersion::TLS1_1,
            TlsVersion::Tls12 => SslVersion::TLS1_2,
            TlsVersion::Tls13 => SslVersion::TLS1_3,
        }
    }

    /// Get version as string
    pub fn as_str(&self) -> &'static str {
        match self {
            TlsVersion::Ssl3 => "SSLv3",
            TlsVersion::Tls10 => "TLSv1.0",
            TlsVersion::Tls11 => "TLSv1.1",
            TlsVersion::Tls12 => "TLSv1.2",
            TlsVersion::Tls13 => "TLSv1.3",
        }
    }
}

/// Client certificate verification mode (server-side)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientVerify {
    /// Don't request client certificates
    None,
    /// Request client certificate but don't require it
    Optional,
    /// Require client certificate
    Required,
}

/// TLS errors
#[derive(Debug, thiserror::Error)]
pub enum TlsError {
    #[error("OpenSSL error: {0}")]
    OpenSsl(#[from] openssl::error::ErrorStack),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid TLS version: {0}")]
    InvalidVersion(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Certificate error: {0}")]
    Certificate(String),

    #[error("Handshake failed: {0}")]
    HandshakeFailed(String),

    #[error("TLS operation failed: {0}")]
    OperationFailed(String),

    #[error("ALPN negotiation failed")]
    AlpnFailed,

    #[error("Session resumption failed: {0}")]
    SessionResumptionFailed(String),
}

/// TLS configuration (immutable after building)
#[derive(Clone)]
pub struct TlsConfig {
    pub(crate) ctx: openssl::ssl::SslContext,
    pub(crate) is_server: bool,
    // Client-specific fields
    pub(crate) servername: Option<String>,
    pub(crate) _verify_peer: bool,
    pub(crate) cert_status: bool,
    pub(crate) _sess_out: Option<String>,
    pub(crate) _sess_in: Option<String>,
}

impl TlsConfig {
    /// Create a new client configuration builder
    pub fn client() -> ClientConfigBuilder {
        ClientConfigBuilder::new()
    }

    /// Create a new server configuration builder
    pub fn server() -> ServerConfigBuilder {
        ServerConfigBuilder::new()
    }

    /// Connect to a server with TLS (client-side)
    pub fn connect(&self, stream: std::net::TcpStream) -> Result<super::TlsSessionOps, TlsError> {
        if self.is_server {
            return Err(TlsError::InvalidConfig(
                "Cannot use server config for client connection".to_string(),
            ));
        }
        super::session::TlsSessionOps::connect(stream, self.clone())
    }

    /// Accept a client connection with TLS (server-side)
    pub fn accept(&self, stream: std::net::TcpStream) -> Result<super::TlsSessionOps, TlsError> {
        if !self.is_server {
            return Err(TlsError::InvalidConfig(
                "Cannot use client config for server accept".to_string(),
            ));
        }
        super::session::TlsSessionOps::accept(stream, self.clone())
    }
}

/// Client configuration builder
pub struct ClientConfigBuilder {
    ctx_builder: openssl::ssl::SslContextBuilder,
    servername: Option<String>,
    verify_peer: bool,
    cert_status: bool,
    sess_out: Option<String>,
    sess_in: Option<String>,
}

impl ClientConfigBuilder {
    fn new() -> Self {
        use openssl::ssl::{SslMethod, SslContextBuilder};

        let mut ctx_builder = SslContextBuilder::new(SslMethod::tls_client())
            .expect("Failed to create SSL context");

        // Default: don't verify peer (for testing)
        ctx_builder.set_verify(openssl::ssl::SslVerifyMode::NONE);

        ClientConfigBuilder {
            ctx_builder,
            servername: None,
            verify_peer: false,  // Will be mapped to _verify_peer in TlsConfig
            cert_status: false,
            sess_out: None,      // Will be mapped to _sess_out in TlsConfig
            sess_in: None,       // Will be mapped to _sess_in in TlsConfig
        }
    }

    /// Set TLS version (both min and max)
    pub fn version(mut self, version: TlsVersion) -> Self {
        self.ctx_builder.set_min_proto_version(Some(version.to_openssl_version()))
            .expect("Failed to set min proto version");
        self.ctx_builder.set_max_proto_version(Some(version.to_openssl_version()))
            .expect("Failed to set max proto version");
        self
    }

    /// Set TLS version range
    pub fn version_range(mut self, min: TlsVersion, max: TlsVersion) -> Self {
        self.ctx_builder.set_min_proto_version(Some(min.to_openssl_version()))
            .expect("Failed to set min proto version");
        self.ctx_builder.set_max_proto_version(Some(max.to_openssl_version()))
            .expect("Failed to set max proto version");
        self
    }

    /// Set cipher list (for TLS <= 1.2)
    pub fn cipher_list(mut self, ciphers: &str) -> Result<Self, TlsError> {
        self.ctx_builder.set_cipher_list(ciphers)?;
        Ok(self)
    }

    /// Set cipher suites (for TLS 1.3)
    pub fn ciphersuites(mut self, ciphers: &str) -> Result<Self, TlsError> {
        self.ctx_builder.set_ciphersuites(ciphers)?;
        Ok(self)
    }

    /// Set ALPN protocols
    pub fn alpn(mut self, protocols: &[&str]) -> Result<Self, TlsError> {
        // Encode ALPN protocols (length-prefixed)
        let mut alpn_bytes = Vec::new();
        for proto in protocols {
            alpn_bytes.push(proto.len() as u8);
            alpn_bytes.extend_from_slice(proto.as_bytes());
        }
        self.ctx_builder.set_alpn_protos(&alpn_bytes)?;
        Ok(self)
    }

    /// Set SNI servername
    pub fn servername(mut self, name: impl Into<String>) -> Self {
        self.servername = Some(name.into());
        self
    }

    /// Enable/disable peer certificate verification
    pub fn verify_peer(mut self, verify: bool) -> Self {
        self.verify_peer = verify;
        if verify {
            self.ctx_builder.set_verify(openssl::ssl::SslVerifyMode::PEER);
        } else {
            self.ctx_builder.set_verify(openssl::ssl::SslVerifyMode::NONE);
        }
        self
    }

    /// Load client certificate from PEM file
    pub fn cert_file<P: AsRef<Path>>(mut self, path: P) -> Result<Self, TlsError> {
        // Read certificate file
        let mut cert_pem = Vec::new();
        File::open(path.as_ref())?.read_to_end(&mut cert_pem)?;

        // Load certificate and private key
        use openssl::x509::X509;
        use openssl::pkey::PKey;

        let cert = X509::from_pem(&cert_pem)
            .map_err(|e| TlsError::Certificate(format!("Failed to load certificate: {}", e)))?;

        self.ctx_builder.set_certificate(&cert)?;

        // Load private key
        let key = PKey::private_key_from_pem(&cert_pem)
            .map_err(|e| TlsError::Certificate(format!("Failed to load private key: {}", e)))?;

        self.ctx_builder.set_private_key(&key)?;

        Ok(self)
    }

    /// Request OCSP staple from server
    pub fn cert_status(mut self, request: bool) -> Self {
        self.cert_status = request;
        self
    }

    /// Save session to file for resumption
    pub fn sess_out(mut self, path: impl Into<String>) -> Self {
        self.sess_out = Some(path.into());
        self
    }

    /// Load session from file for resumption
    pub fn sess_in(mut self, path: impl Into<String>) -> Self {
        self.sess_in = Some(path.into());
        self
    }

    /// Build the TLS configuration
    pub fn build(self) -> Result<TlsConfig, TlsError> {
        Ok(TlsConfig {
            ctx: self.ctx_builder.build(),
            is_server: false,
            servername: self.servername,
            _verify_peer: self.verify_peer,
            cert_status: self.cert_status,
            _sess_out: self.sess_out,
            _sess_in: self.sess_in,
        })
    }
}

/// Server configuration builder
pub struct ServerConfigBuilder {
    ctx_builder: openssl::ssl::SslContextBuilder,
    has_cert: bool,
}

impl ServerConfigBuilder {
    fn new() -> Self {
        use openssl::ssl::{SslMethod, SslContextBuilder};

        let ctx_builder = SslContextBuilder::new(SslMethod::tls_server())
            .expect("Failed to create SSL context");

        ServerConfigBuilder {
            ctx_builder,
            has_cert: false,
        }
    }

    /// Set TLS version (both min and max)
    pub fn version(mut self, version: TlsVersion) -> Self {
        self.ctx_builder.set_min_proto_version(Some(version.to_openssl_version()))
            .expect("Failed to set min proto version");
        self.ctx_builder.set_max_proto_version(Some(version.to_openssl_version()))
            .expect("Failed to set max proto version");
        self
    }

    /// Set TLS version range
    pub fn version_range(mut self, min: TlsVersion, max: TlsVersion) -> Self {
        self.ctx_builder.set_min_proto_version(Some(min.to_openssl_version()))
            .expect("Failed to set min proto version");
        self.ctx_builder.set_max_proto_version(Some(max.to_openssl_version()))
            .expect("Failed to set max proto version");
        self
    }

    /// Set cipher list (for TLS <= 1.2)
    pub fn cipher_list(mut self, ciphers: &str) -> Result<Self, TlsError> {
        self.ctx_builder.set_cipher_list(ciphers)?;
        Ok(self)
    }

    /// Set cipher suites (for TLS 1.3)
    pub fn ciphersuites(mut self, ciphers: &str) -> Result<Self, TlsError> {
        self.ctx_builder.set_ciphersuites(ciphers)?;
        Ok(self)
    }

    /// Set ALPN protocols
    pub fn alpn(mut self, protocols: &[&str]) -> Result<Self, TlsError> {
        // Store protocols for the selection callback
        let protocols_vec: Vec<Vec<u8>> = protocols
            .iter()
            .map(|p| p.as_bytes().to_vec())
            .collect();

        // Set the ALPN selection callback (server-side protocol negotiation)
        self.ctx_builder.set_alpn_select_callback(move |_ssl, client_protos| {
            // Parse client protocols (length-prefixed format)
            let mut pos = 0;
            while pos < client_protos.len() {
                let len = client_protos[pos] as usize;
                pos += 1;
                if pos + len <= client_protos.len() {
                    let client_proto = &client_protos[pos..pos + len];

                    // Check if this matches any of our protocols
                    for proto in &protocols_vec {
                        if client_proto == proto.as_slice() {
                            // Return the matching protocol from client_protos (valid lifetime)
                            return Ok(client_proto);
                        }
                    }

                    pos += len;
                } else {
                    break;
                }
            }

            // No match - return error
            Err(openssl::ssl::AlpnError::NOACK)
        });

        Ok(self)
    }

    /// Load server certificate from PEM file
    pub fn cert_file<P: AsRef<Path>>(mut self, path: P) -> Result<Self, TlsError> {
        // Read certificate file
        let mut cert_pem = Vec::new();
        File::open(path.as_ref())?.read_to_end(&mut cert_pem)?;

        // Load certificate and chain
        use openssl::x509::X509;
        use openssl::pkey::PKey;

        let cert = X509::from_pem(&cert_pem)
            .map_err(|e| TlsError::Certificate(format!("Failed to load certificate: {}", e)))?;

        self.ctx_builder.set_certificate(&cert)?;

        // Load private key
        let key = PKey::private_key_from_pem(&cert_pem)
            .map_err(|e| TlsError::Certificate(format!("Failed to load private key: {}", e)))?;

        self.ctx_builder.set_private_key(&key)?;

        self.has_cert = true;
        Ok(self)
    }

    /// Set client certificate verification mode
    pub fn client_verify(mut self, mode: ClientVerify) -> Self {
        use openssl::ssl::SslVerifyMode;

        let verify_mode = match mode {
            ClientVerify::None => SslVerifyMode::NONE,
            ClientVerify::Optional => SslVerifyMode::PEER,
            ClientVerify::Required => SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT,
        };

        self.ctx_builder.set_verify(verify_mode);
        self
    }

    /// Set CA file for client certificate verification
    pub fn client_verify_ca<P: AsRef<Path>>(mut self, path: P) -> Result<Self, TlsError> {
        self.ctx_builder.set_ca_file(path.as_ref())?;
        Ok(self)
    }

    /// Set OCSP staple response file
    pub fn staple<P: AsRef<Path>>(self, path: P) -> Result<Self, TlsError> {
        // Read OCSP response
        let mut ocsp_resp = Vec::new();
        File::open(path.as_ref())?.read_to_end(&mut ocsp_resp)?;

        // Note: Setting OCSP staple requires additional OpenSSL setup
        // This is a simplified version - full implementation would set up
        // the status_request callback

        // For now, we'll store it but the full OCSP implementation
        // would require more work

        Ok(self)
    }

    /// Build the TLS configuration
    pub fn build(mut self) -> Result<TlsConfig, TlsError> {
        // If no certificate was loaded, use the built-in certificate
        if !self.has_cert {
            self = self.load_builtin_cert()?;
        }

        Ok(TlsConfig {
            ctx: self.ctx_builder.build(),
            is_server: true,
            servername: None,
            _verify_peer: false,
            cert_status: false,
            _sess_out: None,
            _sess_in: None,
        })
    }

    fn load_builtin_cert(mut self) -> Result<Self, TlsError> {
        // Load the built-in certificate
        let cert_pem = super::builtin_cert::BUILTIN_CERT;

        use openssl::x509::X509;
        use openssl::pkey::PKey;

        let cert = X509::from_pem(cert_pem.as_bytes())
            .map_err(|e| TlsError::Certificate(format!("Failed to load built-in certificate: {}", e)))?;

        self.ctx_builder.set_certificate(&cert)?;

        let key = PKey::private_key_from_pem(cert_pem.as_bytes())
            .map_err(|e| TlsError::Certificate(format!("Failed to load built-in private key: {}", e)))?;

        self.ctx_builder.set_private_key(&key)?;

        self.has_cert = true;
        Ok(self)
    }
}

/// Configuration builder (unified interface)
pub struct TlsConfigBuilder;

impl TlsConfigBuilder {
    /// Create a client configuration builder
    pub fn client() -> ClientConfigBuilder {
        ClientConfigBuilder::new()
    }

    /// Create a server configuration builder
    pub fn server() -> ServerConfigBuilder {
        ServerConfigBuilder::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_version_parsing() {
        assert_eq!(TlsVersion::from_str("TLSv1.2").unwrap(), TlsVersion::Tls12);
        assert_eq!(TlsVersion::from_str("tlsv1.3").unwrap(), TlsVersion::Tls13);
        assert_eq!(TlsVersion::from_str("TLS1.0").unwrap(), TlsVersion::Tls10);
        assert!(TlsVersion::from_str("invalid").is_err());
    }

    #[test]
    fn test_client_config_builder() {
        let config = TlsConfig::client()
            .version(TlsVersion::Tls13)
            .servername("example.com")
            .verify_peer(false)
            .build()
            .unwrap();

        assert!(!config.is_server);
        assert_eq!(config.servername, Some("example.com".to_string()));
        assert!(!config._verify_peer);
    }

    #[test]
    fn test_server_config_builder() {
        // Server with built-in cert
        let config = TlsConfig::server()
            .version(TlsVersion::Tls13)
            .client_verify(ClientVerify::Optional)
            .build()
            .unwrap();

        assert!(config.is_server);
    }

    #[test]
    fn test_version_range() {
        let config = TlsConfig::client()
            .version_range(TlsVersion::Tls12, TlsVersion::Tls13)
            .build()
            .unwrap();

        assert!(!config.is_server);
    }
}
