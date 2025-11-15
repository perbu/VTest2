# VTest2 - Rust HTTP Testing Library

A modern HTTP/HTTPS testing library written in Rust, providing low-level control over HTTP/1.1 and HTTP/2 protocol interactions for comprehensive testing.

## Features

### HTTP/2 Implementation
- **Complete protocol support** - All HTTP/2 frame types (DATA, HEADERS, SETTINGS, PING, GOAWAY, etc.)
- **Low-level frame control** - Direct frame construction for testing edge cases and protocol violations
- **Flow control** - Connection and stream-level window management with violation detection
- **Stream multiplexing** - Multiple concurrent streams per connection
- **HPACK compression** - Header compression/decompression using the hpack crate
- **Server push** - PUSH_PROMISE frame support
- **TLS with ALPN** - HTTP/2 over TLS with protocol negotiation

### HTTP/1.1 Implementation
- **Full HTTP/1.1 support** - Request/response handling with keep-alive
- **Chunked encoding** - Transfer-encoding: chunked support
- **Large body handling** - Efficient streaming for large payloads
- **Header management** - Case-insensitive header access

### TLS Support
- **TLS 1.0 - 1.3** - Full TLS version range support
- **ALPN negotiation** - Application-Layer Protocol Negotiation for HTTP/2
- **Certificate management** - Client and server certificates
- **Session resumption** - TLS session caching
- **OCSP stapling** - Certificate status checking
- **SNI support** - Server Name Indication

### Network Layer
- **TCP connection management** - IPv4 and IPv6 support
- **Socket options** - Timeouts, keep-alive, linger, etc.
- **DNS resolution** - Hostname to IP resolution
- **Connection pooling** - Reusable connections

## Building

```bash
# Build the library
cargo build

# Build with optimizations
cargo build --release

# Run all tests (58 integration tests + unit tests)
cargo test

# Run specific test suites
cargo test --test h2_integration        # HTTP/2 tests
cargo test --test h2_server_integration # HTTP/2 server tests
cargo test --test alpn_integration      # ALPN negotiation tests
cargo test --test http_integration      # HTTP/1.1 tests
cargo test --test network_integration_tests  # Network layer tests
```

## Testing

**Test Coverage:** 58/58 integration tests passing (100%)

- HTTP/2 Core: 24 tests
- HTTP/2 Server: 12 tests
- ALPN: 7 tests
- HTTP/1.1: 6 tests
- Network: 9 tests

## Performance Benchmarks

```bash
# Run HTTP/2 performance benchmarks
cargo bench --bench h2_performance

# View results
open target/criterion/report/index.html
```

## Usage Examples

### HTTP/2 Client

```rust
use vtest2::http::h2::{H2Client, H2ClientBuilder};
use vtest2::http::tls::{TlsConfig, TlsVersion};
use std::net::TcpStream;

// Create TLS config with ALPN for HTTP/2
let tls_config = TlsConfig::client()
    .version(TlsVersion::Tls13)
    .servername("example.com")
    .alpn(&["h2"])
    .build()?;

// Connect with TLS
let tcp_stream = TcpStream::connect("example.com:443")?;
let tls_session = tls_config.connect(tcp_stream)?;

// Create HTTP/2 client
let mut client = H2ClientBuilder::new()
    .initial_window_size(65535)
    .build(tls_session)?;

// Send request
client.connect()?;
let response = client.get("/")?;
println!("Status: {}", response.status());
```

### HTTP/2 Server

```rust
use vtest2::http::h2::{H2Server, H2ServerBuilder};
use vtest2::http::tls::{TlsConfig, TlsVersion};
use std::net::TcpListener;
use bytes::Bytes;

// Create TLS config
let tls_config = TlsConfig::server()
    .cert_file("server.pem")?
    .version(TlsVersion::Tls13)
    .alpn(&["h2"])
    .build()?;

// Accept connection
let listener = TcpListener::bind("127.0.0.1:8443")?;
let (tcp_stream, _) = listener.accept()?;
let tls_session = tls_config.accept(tcp_stream)?;

// Create HTTP/2 server
let mut server = H2ServerBuilder::new()
    .max_concurrent_streams(100)
    .build(tls_session)?;

// Accept HTTP/2 connection
server.accept()?;

// Process request
let request = server.recv_request()?;
println!("Received {} {}", request.method(), request.path());

// Send response
server.send_response(
    request.stream_id,
    200,
    &[("content-type", "text/plain")],
    Bytes::from("Hello, HTTP/2!")
)?;
```

## Documentation

- **`HTTP2.md`** - Comprehensive HTTP/2 implementation guide with examples
- **`TLS-IMPL.md`** - TLS support and configuration documentation
- **`CLAUDE.md`** - Architecture details and development guide
- **`HTTP2_VALIDATION_REPORT.md`** - Validation report showing 100% completion

## Architecture

The library is organized into clean, modular components:

```
src/
├── http/
│   ├── h2/              # HTTP/2 implementation
│   │   ├── client.rs    # Client (572 lines)
│   │   ├── server.rs    # Server (664 lines)
│   │   ├── codec.rs     # Frame encoding/decoding
│   │   ├── frames.rs    # Frame type definitions
│   │   ├── stream.rs    # Stream state management
│   │   ├── flow_control.rs # Flow control
│   │   └── settings.rs  # SETTINGS management
│   ├── tls/             # TLS support
│   ├── client.rs        # HTTP/1.1 client
│   ├── server.rs        # HTTP/1.1 server
│   └── session.rs       # Session operations abstraction
├── net/                 # Network layer
│   ├── tcp.rs           # TCP connections
│   ├── addr.rs          # Address handling
│   └── resolver.rs      # DNS resolution
└── lib.rs               # Public API
```

## Dependencies

- **bytes** - Zero-copy byte buffers
- **hpack** - HTTP/2 header compression
- **openssl** - TLS support
- **socket2** - Low-level socket operations
- **thiserror** - Error handling

Development dependencies:
- **criterion** - Performance benchmarking
- **tempfile** - Temporary file handling for tests

## License

See LICENSE file.

## Status

**Production Ready** - 100% complete implementation with comprehensive test coverage.

- ✅ HTTP/2 client and server fully implemented
- ✅ All 58 integration tests passing
- ✅ ALPN negotiation working
- ✅ Flow control validated
- ✅ Error handling comprehensive
- ✅ Performance benchmarks created
