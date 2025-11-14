//! Certificate handling and parsing
//!
//! This module provides functionality for parsing and extracting information
//! from X.509 certificates.

use openssl::x509::X509;
use openssl::nid::Nid;

/// Certificate information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertInfo {
    /// Certificate subject (Common Name)
    pub subject: String,
    /// Certificate issuer (Common Name)
    pub issuer: String,
    /// Subject Alternative Names (DNS names and IP addresses)
    pub subject_alt_names: Vec<String>,
}

impl CertInfo {
    /// Extract certificate information from an X.509 certificate
    pub fn from_x509(cert: &X509) -> Self {
        CertInfo {
            subject: Self::get_cn(cert.subject_name()),
            issuer: Self::get_cn(cert.issuer_name()),
            subject_alt_names: Self::get_subject_alt_names(cert),
        }
    }

    /// Extract certificate information from an X.509 certificate reference
    pub fn from_x509_ref(cert: &openssl::x509::X509Ref) -> Self {
        CertInfo {
            subject: Self::get_cn(cert.subject_name()),
            issuer: Self::get_cn(cert.issuer_name()),
            subject_alt_names: Self::get_subject_alt_names_ref(cert),
        }
    }

    /// Get Common Name from X509_NAME
    fn get_cn(name: &openssl::x509::X509NameRef) -> String {
        name.entries_by_nid(Nid::COMMONNAME)
            .next()
            .and_then(|entry| entry.data().as_utf8().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "<undef>".to_string())
    }

    /// Get Subject Alternative Names
    fn get_subject_alt_names(cert: &X509) -> Vec<String> {
        Self::get_subject_alt_names_impl(cert)
    }

    /// Get Subject Alternative Names from X509Ref
    fn get_subject_alt_names_ref(cert: &openssl::x509::X509Ref) -> Vec<String> {
        Self::get_subject_alt_names_impl(cert)
    }

    /// Get Subject Alternative Names implementation (works with both X509 and X509Ref)
    fn get_subject_alt_names_impl<T: AsRef<openssl::x509::X509Ref>>(cert: &T) -> Vec<String> {
        let mut names = Vec::new();
        let cert_ref = cert.as_ref();

        if let Some(san_ext) = cert_ref.subject_alt_names() {
            for name in san_ext {
                if let Some(dns) = name.dnsname() {
                    names.push(format!("DNS:{}", dns));
                } else if let Some(ip) = name.ipaddress() {
                    if ip.len() == 4 {
                        // IPv4
                        names.push(format!(
                            "IP:{}.{}.{}.{}",
                            ip[0], ip[1], ip[2], ip[3]
                        ));
                    } else if ip.len() == 16 {
                        // IPv6 - convert to standard format
                        names.push(format!(
                            "IP:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}",
                            ip[0], ip[1], ip[2], ip[3], ip[4], ip[5], ip[6], ip[7],
                            ip[8], ip[9], ip[10], ip[11], ip[12], ip[13], ip[14], ip[15]
                        ));
                    }
                }
            }
        }

        names
    }
}

/// Extract certificate chain information from SSL connection
pub fn get_cert_chain(ssl: &openssl::ssl::SslRef) -> Vec<CertInfo> {
    let mut chain = Vec::new();

    // Get peer certificate (index 0)
    if let Some(peer_cert) = ssl.peer_certificate() {
        chain.push(CertInfo::from_x509(&peer_cert));
    }

    // Get certificate chain (index 1+)
    if let Some(cert_chain) = ssl.peer_cert_chain() {
        for cert in cert_chain {
            chain.push(CertInfo::from_x509_ref(cert));
        }
    }

    chain
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::builtin_cert::BUILTIN_CERT;

    #[test]
    fn test_cert_info_from_builtin() {
        let cert = X509::from_pem(BUILTIN_CERT.as_bytes()).unwrap();
        let info = CertInfo::from_x509(&cert);

        assert_eq!(info.subject, "example.com");
        assert_eq!(info.issuer, "example.com"); // Self-signed

        // Check SANs
        assert!(info.subject_alt_names.contains(&"DNS:example.com".to_string()));
        assert!(info.subject_alt_names.contains(&"DNS:*.example.com".to_string()));
    }

    #[test]
    fn test_get_cn() {
        let cert = X509::from_pem(BUILTIN_CERT.as_bytes()).unwrap();
        let subject = CertInfo::get_cn(cert.subject_name());
        assert_eq!(subject, "example.com");
    }

    #[test]
    fn test_get_subject_alt_names() {
        let cert = X509::from_pem(BUILTIN_CERT.as_bytes()).unwrap();
        let sans = CertInfo::get_subject_alt_names(&cert);

        assert_eq!(sans.len(), 2);
        assert!(sans.contains(&"DNS:example.com".to_string()));
        assert!(sans.contains(&"DNS:*.example.com".to_string()));
    }
}
