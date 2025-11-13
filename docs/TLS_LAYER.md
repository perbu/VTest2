# TLS Layer Implementation for VTest2 (Rust)

## Overview

This document describes the TLS (Transport Layer Security) implementation for the Rust port of VTest2. The TLS layer enables encrypted HTTPS connections for testing HTTP clients, servers, and proxies.

## Architecture

The TLS implementation follows the **session operations abstraction pattern** established in the HTTP layer:

```
Plain TCP                    TLS
-----------                 -----
TcpStream  →  FdSessionOps   →  HttpSession  →  HttpClient/Server
TcpStream  →  TlsSessionOps  →  HttpSession  →  HttpClient/Server
```

### Key Components

1. **TlsConfig** - Immutable TLS configuration (client or server)
2. **TlsSessionOps** - Implements `SessionOps` trait for TLS-encrypted I/O
3. **TlsVars** - TLS variables for test expectations
4. **CertInfo** - Certificate information extraction

All HTTP I/O code remains unchanged - it transparently uses either plain or TLS operations based on the SessionOps implementation.

## Module Structure

```
src/http/tls/
├── mod.rs           - Public TLS API and exports
├── config.rs        - TLS configuration builders
├── session.rs       - TLS SessionOps implementation
├── handshake.rs     - TLS handshake utilities
├── cert.rs          - Certificate handling
├── vars.rs          - TLS variables for expect commands
└── builtin_cert.rs  - Embedded default certificate
```

## Features

- **TLS Versions**: TLS 1.0, 1.1, 1.2, 1.3 (depends on OpenSSL version)
- **Certificate Management**: Load certificates from PEM files or use built-in certificate
- **ALPN**: Application-Layer Protocol Negotiation (e.g., h2, http/1.1)
- **SNI**: Server Name Indication for virtual hosting
- **Client Certificate Verification**: None, Optional, or Required
- **Session Operations**: Transparent switching between plain TCP and TLS
- **TLS Variables**: Extensive variables for test expectations

## Usage Examples

### Basic Client Configuration

```rust
use vtest2::http::tls::{TlsConfig, TlsVersion};
use std::net::TcpStream;

// Create client configuration
let tls_config = TlsConfig::client()
    .version(TlsVersion::Tls13)
    .servername("example.com")
    .verify_peer(false)
    .build()
    .unwrap();

// Connect to server
let tcp_stream = TcpStream::connect("example.com:443").unwrap();
let tls_session = tls_config.connect(tcp_stream).unwrap();

// Use with HTTP client
let mut client = HttpClient::from_tls(tls_session);
```

### Basic Server Configuration

```rust
use vtest2::http::tls::{TlsConfig, TlsVersion, ClientVerify};
use std::net::TcpListener;

// Create server configuration (uses built-in certificate)
let tls_config = TlsConfig::server()
    .version_range(TlsVersion::Tls12, TlsVersion::Tls13)
    .client_verify(ClientVerify::Optional)
    .build()
    .unwrap();

// Accept client connection
let listener = TcpListener::bind("127.0.0.1:443").unwrap();
let (tcp_stream, _) = listener.accept().unwrap();
let tls_session = tls_config.accept(tcp_stream).unwrap();

// Use with HTTP server
let mut server = HttpServer::from_tls(tls_session);
```

### Advanced Client Configuration

```rust
use vtest2::http::tls::{TlsConfig, TlsVersion};

let tls_config = TlsConfig::client()
    .version_range(TlsVersion::Tls12, TlsVersion::Tls13)
    .servername("api.example.com")
    .verify_peer(true)
    .cert_file("/path/to/client-cert.pem").unwrap()
    .cipher_list("ECDHE-RSA-AES128-GCM-SHA256").unwrap()
    .alpn(&["h2", "http/1.1"]).unwrap()
    .build()
    .unwrap();
```

### Advanced Server Configuration

```rust
use vtest2::http::tls::{TlsConfig, TlsVersion, ClientVerify};

let tls_config = TlsConfig::server()
    .cert_file("/path/to/server-bundle.pem").unwrap()
    .version_range(TlsVersion::Tls12, TlsVersion::Tls13)
    .cipher_list("ECDHE-RSA-AES256-GCM-SHA384").unwrap()
    .ciphersuites("TLS_AES_256_GCM_SHA384").unwrap()
    .alpn(&["h2", "http/1.1"]).unwrap()
    .client_verify(ClientVerify::Required)
    .client_verify_ca("/path/to/ca-bundle.pem").unwrap()
    .build()
    .unwrap();
```

## TLS Configuration Options

### Common Options (Client & Server)

| Option | Type | Description |
|--------|------|-------------|
| `version(v)` | `TlsVersion` | Set both min and max TLS version |
| `version_range(min, max)` | `TlsVersion, TlsVersion` | Set TLS version range |
| `cipher_list(ciphers)` | `&str` | Cipher list for TLS ≤ 1.2 (colon-separated) |
| `ciphersuites(ciphers)` | `&str` | Cipher suites for TLS 1.3 (colon-separated) |
| `alpn(protocols)` | `&[&str]` | ALPN protocol list (e.g., `&["h2", "http/1.1"]`) |
| `cert_file(path)` | `Path` | Load certificate/key bundle from PEM file |

### Client-Only Options

| Option | Type | Description |
|--------|------|-------------|
| `servername(host)` | `String` | SNI hostname |
| `verify_peer(enabled)` | `bool` | Enable/disable peer certificate verification |
| `cert_status(enabled)` | `bool` | Request OCSP staple from server |
| `sess_out(path)` | `String` | Save TLS session for resumption |
| `sess_in(path)` | `String` | Resume TLS session from file |

### Server-Only Options

| Option | Type | Description |
|--------|------|-------------|
| `client_verify(mode)` | `ClientVerify` | Client certificate verification (None, Optional, Required) |
| `client_verify_ca(path)` | `Path` | CA bundle for client certificate verification |
| `staple(path)` | `Path` | Provide OCSP staple response |

## TLS Variables

After a TLS handshake, the following variables are available for test expectations:

### Basic Variables

| Variable | Type | Description |
|----------|------|-------------|
| `tls.version` | `String` | Negotiated TLS version (e.g., "TLSv1.3") |
| `tls.cipher` | `String` | Negotiated cipher suite |
| `tls.failed` | `bool` | Whether handshake or I/O failed |
| `tls.sess_reused` | `bool` | Whether session was resumed |

### Client-Side Variables

| Variable | Type | Description |
|----------|------|-------------|
| `tls.servername` | `Option<String>` | SNI hostname |
| `tls.alpn` | `Option<String>` | Negotiated ALPN protocol |

### Certificate Variables

| Variable | Type | Description |
|----------|------|-------------|
| `tls.cert.subject` | `String` | Peer certificate subject (CN) |
| `tls.cert.issuer` | `String` | Peer certificate issuer |
| `tls.cert.subject_alt_names` | `String` | Subject Alternative Names (comma-separated) |
| `tls.cert[N].subject` | `String` | Certificate N subject (0 = peer, 1+ = chain) |
| `tls.cert[N].issuer` | `String` | Certificate N issuer |

### Accessing Variables

```rust
let tls_session = tls_config.connect(tcp_stream).unwrap();
let vars = tls_session.vars();

println!("TLS Version: {}", vars.version);
println!("Cipher: {}", vars.cipher);
println!("Failed: {}", vars.failed);

if let Some(cert) = vars.cert(0) {
    println!("Peer CN: {}", cert.subject);
    println!("Issuer: {}", cert.issuer);
    println!("SANs: {}", cert.subject_alt_names.join(", "));
}

// Or use the get() method for string lookups
if let Some(version) = vars.get("tls.version") {
    println!("Version via get(): {}", version);
}
```

## Built-in Certificate

The TLS implementation includes a built-in self-signed certificate (CN=example.com) that is automatically used for servers when no certificate is specified.

**Certificate Details:**
- **Common Name**: example.com
- **Organization**: Varnish Software AS
- **Country**: NO
- **Subject Alternative Names**: example.com, *.example.com
- **Valid From**: 2020-01-30
- **Valid To**: 2047-06-17

**Usage:**

```rust
// Server automatically uses built-in certificate
let tls_config = TlsConfig::server()
    .version(TlsVersion::Tls13)
    .build()
    .unwrap();
```

## Implementation Details

### OpenSSL Crate

The Rust implementation uses the `openssl` crate (version 0.10) which provides Rust bindings to OpenSSL. This ensures:

- Maximum compatibility with the C implementation
- Feature parity with the original TLS support
- Access to all OpenSSL features (TLS 1.3, ALPN, OCSP, etc.)

### Session Operations Pattern

The TLS implementation follows the established session operations pattern:

```rust
pub trait SessionOps {
    fn poll(&self, events: PollEvents, timeout: Option<Duration>) -> Result<bool>;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    fn write(&mut self, buf: &[u8]) -> Result<usize>;
    fn close(&mut self) -> Result<()>;
}
```

The `TlsSessionOps` struct implements this trait, wrapping an `openssl::ssl::SslStream<TcpStream>`. All HTTP I/O operations work transparently with both plain and TLS connections.

### Handshake Process

The handshake is performed synchronously in blocking mode:

1. Create `Ssl` object from `SslContext`
2. Configure SSL options (SNI, ALPN, etc.)
3. Call `Ssl::connect()` or `Ssl::accept()`
4. Extract TLS variables from the established connection
5. Return `TlsSessionOps` ready for use

## Testing

### Unit Tests

The TLS module includes comprehensive unit tests:

```rust
// Run all TLS tests
cargo test --lib http::tls

// Run specific test
cargo test --lib http::tls::session::tests::test_tls_client_server_handshake
```

### Integration Tests

Complete TLS handshake tests verify:

- Client-server handshake completion
- Data transfer over TLS
- TLS variable population
- Session close handling

Example test:

```rust
#[test]
fn test_tls_client_server_handshake() {
    let server_config = TlsConfig::server()
        .version(TlsVersion::Tls13)
        .build()
        .unwrap();

    let client_config = TlsConfig::client()
        .version(TlsVersion::Tls13)
        .verify_peer(false)
        .build()
        .unwrap();

    // Server and client perform handshake and exchange data
    // ...
}
```

## Differences from C Implementation

### API Style

- **C**: Imperative configuration with command parsing
- **Rust**: Builder pattern with type-safe configuration

### Memory Safety

- **C**: Manual memory management, pointer-based
- **Rust**: Automatic memory management, ownership-based
- **No unsafe code** in TLS configuration and session handling

### Error Handling

- **C**: Return codes and global error state
- **Rust**: Result types and explicit error handling

### Type Safety

The Rust implementation provides compile-time type safety:

```rust
// Won't compile - wrong config type
let client_config = TlsConfig::server().build().unwrap();
client_config.connect(tcp_stream).unwrap(); // Error!

// Correct usage
let client_config = TlsConfig::client().build().unwrap();
client_config.connect(tcp_stream).unwrap(); // OK
```

## Performance Considerations

- **Handshake**: Synchronous, blocking operation
- **I/O**: Standard SSL_read/SSL_write performance
- **Memory**: Similar to C implementation (OpenSSL under the hood)
- **Overhead**: Minimal Rust wrapper overhead

## Troubleshooting

### Common Issues

1. **Handshake Timeout**: Increase timeout or check network connectivity
2. **Certificate Verification Failed**: Ensure proper CA bundle or disable verification for testing
3. **ALPN Negotiation Failed**: Check that both client and server support the same protocols
4. **TLS Version Mismatch**: Ensure version ranges overlap

### Debug Logging

Enable OpenSSL error logging:

```rust
use openssl::error::ErrorStack;

match tls_config.connect(tcp_stream) {
    Ok(session) => { /* ... */ }
    Err(e) => {
        eprintln!("TLS Error: {}", e);
        // OpenSSL error stack is automatically included
    }
}
```

## Future Enhancements

### Planned Features

1. **Async TLS Support**: Non-blocking TLS operations for async runtime
2. **Custom Handshake Callbacks**: For advanced TLS debugging
3. **TLS 1.3 0-RTT**: Early data support
4. **Certificate Pinning**: Pin specific certificates for testing
5. **mTLS Helpers**: Easier mutual TLS setup

### Potential Optimizations

1. **Connection Pooling**: Reuse TLS sessions across tests
2. **Certificate Caching**: Cache parsed certificates
3. **Session Ticket Management**: Better session resumption handling

## References

- [OpenSSL Rust Crate Documentation](https://docs.rs/openssl/)
- [RFC 8446 - TLS 1.3](https://tools.ietf.org/html/rfc8446)
- [RFC 7301 - ALPN](https://tools.ietf.org/html/rfc7301)
- [C Implementation: src/vtc_tls.c](/home/user/VTest2/src/vtc_tls.c)
- [C Documentation: TLS-IMPL.md](/home/user/VTest2/TLS-IMPL.md)

## Contributing

When contributing to the TLS layer:

1. Maintain compatibility with the C implementation
2. Add tests for new features
3. Update this documentation
4. Ensure no unsafe code (unless absolutely necessary)
5. Follow Rust naming conventions and idioms

## License

Same as VTest2 - see main LICENSE file.

---

**Implementation Date**: 2025-11-13
**OpenSSL Version**: 0.10.75
**Status**: Complete and tested
