//! Built-in self-signed certificate
//!
//! This module provides a default self-signed certificate (CN=example.com)
//! for testing without requiring users to provide their own certificates.
//!
//! The certificate is the same one used in the C implementation,
//! ensuring compatibility with existing tests.

/// Built-in self-signed certificate (CN=example.com)
///
/// This certificate is valid from 2020-01-30 to 2047-06-17 and includes:
/// - Common Name (CN): example.com
/// - Subject Alternative Names: example.com, *.example.com
/// - Organization: Varnish Software AS
/// - Country: NO
///
/// The certificate bundle includes both the certificate and private key in PEM format.
pub const BUILTIN_CERT: &str = "\
-----BEGIN CERTIFICATE-----
MIIDwzCCAqugAwIBAgIUe4v+PgBZeohddbh92DAKmy8N6nAwDQYJKoZIhvcNAQEL
BQAwVjELMAkGA1UEBhMCTk8xEzARBgNVBAgMClNvbWUtU3RhdGUxHDAaBgNVBAoM
E1Zhcm5pc2ggU29mdHdhcmUgQVMxFDASBgNVBAMMC2V4YW1wbGUuY29tMB4XDTIw
MDEzMDEwMDMzOFoXDTQ3MDYxNzEwMDMzOFowVjELMAkGA1UEBhMCTk8xEzARBgNV
BAgMClNvbWUtU3RhdGUxHDAaBgNVBAoME1Zhcm5pc2ggU29mdHdhcmUgQVMxFDAS
BgNVBAMMC2V4YW1wbGUuY29tMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKC
AQEA3/STgvtdRnVPnuiONY4ZtUXexHgOUAhiYnm7GuLKrJCqC1DoSwjeA8Fr/sly
nrkS0QdrHDh3tZ/9JO4JUChy+hISBjer32JOpmwwsKyuM4YkQ9YI9NeAJQX4vSeF
krdau2OxuKn9L0e/D8TddzAQ39AOjrE+Y2lCzvoGF2cEesxMNS66JStDFR2w2I7e
EdTydyXYT7mK6iqhk/3RB3XdwvdQj8DzPQSVFe6/pCa+dzpSSLI8YEHkB8azaz3H
jsFp4flSPJJMX+pChbs8NBtekuHWDIExKIeyIpEBd37eoZR9+41PZJOsvya/JIhR
BmVa/t66NHg8ETqUdZYn35pBwQIDAQABo4GIMIGFMCUGA1UdEQQeMByCC2V4YW1w
bGUuY29tgg0qLmV4YW1wbGUuY29tMB0GA1UdDgQWBBSNwlE7yKISR2VwKF/ODERV
528ppTAfBgNVHSMEGDAWgBSNwlE7yKISR2VwKF/ODERV528ppTAPBgNVHRMBAf8E
BTADAQH/MAsGA1UdDwQEAwIFoDANBgkqhkiG9w0BAQsFAAOCAQEAh9M6yB0avQqL
eXsE9EFINZkWGcMsOexArLAiKfNx5ntXelwfjxRwIgepYE8wTh+YfGwTby3Z8BWP
IVODhu+AH2FlRqw/1y8bo/yf0bcGCu5fj7K3AdjCk03DtbZORtFxQ+5z7DDRxgbV
rqwu3hPBm9FDcOEcaoBZ8tw4Mev4GRVwgIGg46UXHOPuoUwrmIZkHGo6ToqKAwwP
eyyRkeNjytrTN0vnmcAuAeWVwGyfIajhsrM2xN3LLYknUfDQU9+8vQvXl8zlBYX+
nSKLgzg1n8WNWHgDWijIaDrtKT2ejhslR+pHaKMTcBRVErpmWSkJ5zlVdalolTHU
ADuwRXuDUg==
-----END CERTIFICATE-----
-----BEGIN RSA PRIVATE KEY-----
MIIEowIBAAKCAQEA3/STgvtdRnVPnuiONY4ZtUXexHgOUAhiYnm7GuLKrJCqC1Do
SwjeA8Fr/slynrkS0QdrHDh3tZ/9JO4JUChy+hISBjer32JOpmwwsKyuM4YkQ9YI
9NeAJQX4vSeFkrdau2OxuKn9L0e/D8TddzAQ39AOjrE+Y2lCzvoGF2cEesxMNS66
JStDFR2w2I7eEdTydyXYT7mK6iqhk/3RB3XdwvdQj8DzPQSVFe6/pCa+dzpSSLI8
YEHkB8azaz3HjsFp4flSPJJMX+pChbs8NBtekuHWDIExKIeyIpEBd37eoZR9+41P
ZJOsvya/JIhRBmVa/t66NHg8ETqUdZYn35pBwQIDAQABAoIBAFXKKevGAKAp9hso
eLl5Os3e+wwF9W2hGJcijJMrB3p9XDZDgwijV/DWWllar+avfM7H6bcAxpKzu9Q2
vyiOpiS3YWIyV0uWLAzCaxByxbSFEUVPK1UnbDZCiFtlVVyzkjUwZncX3x4KfN08
i53Jst0ZpUnyCbUpMGd7DXRPiT7EZj9ri4C/GA3VK/6zAYjlqXN0S0wcRBSVV26V
5ZUve/daGjmnQu+YYB8Ni/mlph+nhPGVT5uwD/xb+fca6YyAbFKriPJ91lpDqaR9
UqniwpKx6nsnZXFIctjYdqkSHLD1O92vFehHoVDrSQi66CptjqUAB9umkqYqug4t
sQArDjECgYEA/PziahI9pJEYfs5uL93eSKh/v8TmYTP9pCoZE8oy63mZ4mQs0DMV
fU+lMGDpzzFGyda+CBz8I+peNfkvyh742fejGqPUiKGvFNW9HajayRyI8zgxH66/
KCjJJlcgbcWzgwFJwwQvkeLYFyAFCyKjSJf4AQcU4XT2f9TbcNxI9qUCgYEA4p8z
KtdR1C8lnTFYkZxxFkX6jScsHwGRv3ypxGrSYNiSxqyJjm/XYIwi4adgyk4vHoFz
doDtjFmH9Ib7AaI4DLUZSwBobROHxTdEyL4plaQl3iiIT03vxr9zH1xHlMsDctif
tuz0HQ68gC/0DgaySTIk9+SltDH6G6eYOepdT+0CgYAcDl99q/AyI/U3euU1YcGZ
BTbFqaxy8zUZ06FcVHw5KQ8r0Dg4DrI/Z2nGZ7kGRUy4bZw9ghlkUkWIbs4h+DVY
1uG7vpd/X47vHJUQiP1aeFOnxX+NJ/ADICLOobLy+Y3i5W2stvYfk6yrQ93LUlgR
YOkcFBD4v+PmYVDEv2lIEQKBgCFx7VM9Q85UxvBUAAY9WFM5MKj0RwasbJ4d/9AF
E9dHHyJDBGoJB3gwNlWnJhm1QC74W9n5XRWBgRcNdK3hCvSVJY50GPVAFKF+bqBR
sEFtYElRIgzSK7jhOFRAgi/rZi7k2W1duwkuy5L/gL0xL86tn9cV336ggZDjQwwJ
EoxhAoGBAIqQzGle4KV/TujqAEoF+m1b2/UWVb5sV6PFnJCwP9Xp0OtX2MRLj4iV
kc1i5xRzIQKeSt7XW4fCF8rgvPmPXb88h8F5/ANg1/sKd5tzRHXA/2B7cMIEv1rb
7aqpn0Tft2l37ZBkihoceb7A63ec2C6jjeTEzYgaCJibxkETS2QO
-----END RSA PRIVATE KEY-----
";

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::x509::X509;
    use openssl::pkey::PKey;

    #[test]
    fn test_builtin_cert_loads() {
        // Test that the certificate can be loaded
        let cert = X509::from_pem(BUILTIN_CERT.as_bytes()).unwrap();

        // Verify CN
        let subject = cert.subject_name();
        let cn = subject.entries_by_nid(openssl::nid::Nid::COMMONNAME)
            .next()
            .unwrap();
        let cn_str = cn.data().as_utf8().unwrap().to_string();
        assert_eq!(cn_str, "example.com");
    }

    #[test]
    fn test_builtin_private_key_loads() {
        // Test that the private key can be loaded
        let key = PKey::private_key_from_pem(BUILTIN_CERT.as_bytes()).unwrap();

        // Verify it's an RSA key
        assert!(key.rsa().is_ok());
    }
}
