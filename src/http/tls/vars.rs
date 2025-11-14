//! TLS variables for expect commands
//!
//! This module provides TLS variables that can be used in test expectations.
//! These variables are populated after a TLS handshake and provide information
//! about the negotiated connection.

use super::cert::{CertInfo, get_cert_chain};
use openssl::ssl::SslRef;

/// TLS variables available after handshake
#[derive(Debug, Clone)]
pub struct TlsVars {
    /// Negotiated TLS version (e.g., "TLSv1.3")
    pub version: String,

    /// Negotiated cipher suite
    pub cipher: String,

    /// SNI servername (client-side)
    pub servername: Option<String>,

    /// Negotiated ALPN protocol
    pub alpn: Option<String>,

    /// Latest TLS alert message
    pub alert: Option<String>,

    /// Whether handshake or I/O failed
    pub failed: bool,

    /// Certificate chain (index 0 is peer cert)
    pub cert_chain: Vec<CertInfo>,

    /// Whether session was resumed
    pub sess_reused: bool,

    /// Whether client requested OCSP staple
    pub staple_requested: bool,

    // OCSP variables (client-only in C implementation)
    /// OCSP certificate status
    pub ocsp_cert_status: Option<String>,

    /// OCSP response status
    pub ocsp_resp_status: Option<String>,

    /// OCSP verification result
    pub ocsp_verify: Option<String>,
}

impl TlsVars {
    /// Create TLS variables from an SSL connection
    pub fn from_ssl(ssl: &SslRef, failed: bool) -> Self {
        let version = if !failed {
            ssl.version_str().to_string()
        } else {
            "<undef>".to_string()
        };

        let cipher = if !failed {
            ssl.current_cipher()
                .map(|c| c.name().to_string())
                .unwrap_or_else(|| "<undef>".to_string())
        } else {
            "<undef>".to_string()
        };

        let servername = if !failed {
            ssl.servername(openssl::ssl::NameType::HOST_NAME)
                .map(|s| s.to_string())
        } else {
            None
        };

        let alpn = if !failed {
            ssl.selected_alpn_protocol()
                .map(|p| String::from_utf8_lossy(p).to_string())
        } else {
            None
        };

        let cert_chain = if !failed {
            get_cert_chain(ssl)
        } else {
            Vec::new()
        };

        let sess_reused = if !failed {
            ssl.session_reused()
        } else {
            false
        };

        // Note: staple_requested requires checking if status_type is set
        // This is a simplified version
        let staple_requested = false; // TODO: Implement proper check

        TlsVars {
            version,
            cipher,
            servername,
            alpn,
            alert: None, // Set by handshake errors or SSL alerts
            failed,
            cert_chain,
            sess_reused,
            staple_requested,
            ocsp_cert_status: None,
            ocsp_resp_status: None,
            ocsp_verify: None,
        }
    }

    /// Get certificate info by index (0 = peer cert, 1+ = chain)
    pub fn cert(&self, index: usize) -> Option<&CertInfo> {
        self.cert_chain.get(index)
    }

    /// Get variable by name (for expect commands)
    pub fn get(&self, name: &str) -> Option<String> {
        match name {
            "tls.version" => Some(self.version.clone()),
            "tls.cipher" => Some(self.cipher.clone()),
            "tls.servername" => self.servername.clone().or(Some("<undef>".to_string())),
            "tls.alpn" => self.alpn.clone().or(Some("<undef>".to_string())),
            "tls.alert" => self.alert.clone().or(Some("<undef>".to_string())),
            "tls.failed" => Some(if self.failed { "true" } else { "false" }.to_string()),
            "tls.sess_reused" => Some(if self.sess_reused { "true" } else { "false" }.to_string()),
            "tls.staple_requested" => Some(if self.staple_requested { "true" } else { "false" }.to_string()),
            _ => {
                // Check for cert.* variables
                if name.starts_with("tls.cert") {
                    self.get_cert_var(name)
                } else {
                    None
                }
            }
        }
    }

    /// Get certificate variable
    fn get_cert_var(&self, name: &str) -> Option<String> {
        // Parse tls.cert[N].field or tls.cert.field (N=0 implicit)
        let remaining = name.strip_prefix("tls.cert")?;

        let (index, field) = if remaining.starts_with('.') {
            // tls.cert.field -> index 0
            (0, remaining.strip_prefix('.')?)
        } else {
            // tls.certN.field -> parse N
            let end_idx = remaining.find('.')?;
            let index_str = &remaining[..end_idx];
            let index: usize = index_str.parse().ok()?;
            let field = remaining[(end_idx + 1)..].trim();
            (index, field)
        };

        let cert = self.cert(index)?;

        match field {
            "subject" => Some(cert.subject.clone()),
            "issuer" => Some(cert.issuer.clone()),
            "subject_alt_names" => {
                if cert.subject_alt_names.is_empty() {
                    Some("<undef>".to_string())
                } else {
                    Some(cert.subject_alt_names.join(", "))
                }
            }
            _ => None,
        }
    }
}

impl Default for TlsVars {
    fn default() -> Self {
        TlsVars {
            version: "<undef>".to_string(),
            cipher: "<undef>".to_string(),
            servername: None,
            alpn: None,
            alert: None,
            failed: true,
            cert_chain: Vec::new(),
            sess_reused: false,
            staple_requested: false,
            ocsp_cert_status: None,
            ocsp_resp_status: None,
            ocsp_verify: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_vars() {
        let vars = TlsVars::default();
        assert_eq!(vars.get("tls.version"), Some("<undef>".to_string()));
        assert_eq!(vars.get("tls.cipher"), Some("<undef>".to_string()));
        assert_eq!(vars.get("tls.failed"), Some("true".to_string()));
        assert_eq!(vars.get("tls.sess_reused"), Some("false".to_string()));
    }

    #[test]
    fn test_cert_var_parsing() {
        use super::super::builtin_cert::BUILTIN_CERT;
        use openssl::x509::X509;

        let cert = X509::from_pem(BUILTIN_CERT.as_bytes()).unwrap();
        let cert_info = CertInfo::from_x509(&cert);

        let mut vars = TlsVars::default();
        vars.cert_chain = vec![cert_info];
        vars.failed = false;

        // Test tls.cert.subject (index 0 implicit)
        assert_eq!(vars.get("tls.cert.subject"), Some("example.com".to_string()));

        // Test tls.cert0.subject (explicit index)
        assert_eq!(vars.get("tls.cert0.subject"), Some("example.com".to_string()));

        // Test tls.cert.issuer
        assert_eq!(vars.get("tls.cert.issuer"), Some("example.com".to_string()));

        // Test tls.cert.subject_alt_names
        let sans = vars.get("tls.cert.subject_alt_names").unwrap();
        assert!(sans.contains("DNS:example.com"));
        assert!(sans.contains("DNS:*.example.com"));
    }

    #[test]
    fn test_boolean_vars() {
        let mut vars = TlsVars::default();
        vars.failed = false;
        vars.sess_reused = true;

        assert_eq!(vars.get("tls.failed"), Some("false".to_string()));
        assert_eq!(vars.get("tls.sess_reused"), Some("true".to_string()));
    }
}
