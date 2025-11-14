# HTTP/2 Implementation in VTest2

This document describes the HTTP/2 implementation in VTest2, ported from the C vtc_http2.c implementation to Rust.

## Overview

VTest2 includes a complete HTTP/2 protocol implementation designed for testing HTTP clients, servers, and proxies. The implementation provides low-level control over frame construction, allowing you to create both valid and intentionally malformed HTTP/2 traffic for comprehensive testing.

## Architecture

The HTTP/2 implementation is organized into several modules:

### Core Modules

- **`h2/frames.rs`** - HTTP/2 frame types (DATA, HEADERS, SETTINGS, PING, etc.)
- **`h2/codec.rs`** - Frame encoding and decoding
- **`h2/stream.rs`** - Stream state management and multiplexing
- **`h2/flow_control.rs`** - Connection and stream-level flow control
- **`h2/settings.rs`** - SETTINGS frame handling and configuration
- **`h2/error.rs`** - Error types and handling
- **`h2/client.rs`** - HTTP/2 client implementation
- **`h2/server.rs`** - HTTP/2 server implementation

### Design Principles

1. **Low-Level Control**: Direct frame construction and manipulation for testing edge cases
2. **Explicit State Management**: Manual stream state transitions for precise control
3. **Testing-Friendly API**: Designed for creating both valid and invalid HTTP/2 traffic
4. **RFC 7540 Compliance**: Implements HTTP/2 specification requirements

## Key Features

### Frame Handling

All HTTP/2 frame types are supported:
- **DATA** - Application data transmission
- **HEADERS** - Header block transmission
- **PRIORITY** - Stream priority information
- **RST_STREAM** - Stream termination
- **SETTINGS** - Connection configuration
- **PUSH_PROMISE** - Server push promises
- **PING** - Connection liveness
- **GOAWAY** - Connection termination
- **WINDOW_UPDATE** - Flow control window updates
- **CONTINUATION** - Header block continuation

### Stream Multiplexing

- Multiple concurrent streams per connection
- Stream ID management (odd for client, even for server)
- Configurable max concurrent streams limit
- Stream state machine (Idle → Open → HalfClosed → Closed)

### Flow Control

- Per-stream and connection-level flow control windows
- Automatic flow control violation detection
- Configurable initial window size
- WINDOW_UPDATE frame generation

### HPACK Compression

- Header compression/decompression using the `hpack` crate
- Dynamic table size management
- Huffman encoding support

## Usage Examples

### Basic HTTP/2 Client

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
let response = client.get("/").await?;
println!("Status: {}", response.status());
println!("Body: {}", response.body_string()?);
```

### Basic HTTP/2 Server

```rust
use vtest2::http::h2::{H2Server, H2ServerBuilder};
use vtest2::http::tls::{TlsConfig, TlsVersion};
use std::net::TcpListener;

// Create TLS config
let tls_config = TlsConfig::server()
    .cert_file("server.pem")?
    .version(TlsVersion::Tls13)
    .alpn(&["h2"])
    .build()?;

// Accept connection
let listener = TcpListener::bind("127.0.0.1:443")?;
let (tcp_stream, _) = listener.accept()?;
let tls_session = tls_config.accept(tcp_stream)?;

// Create HTTP/2 server
let mut server = H2ServerBuilder::new()
    .max_concurrent_streams(100)
    .build(tls_session)?;

// Process request
let request = server.receive_request().await?;
server.send_response(200, &[], b"OK").await?;
```

### Low-Level Frame Construction

```rust
use vtest2::http::h2::frames::*;
use vtest2::http::h2::codec::FrameCodec;
use bytes::Bytes;

// Create a DATA frame with padding
let data = Bytes::from("Hello, HTTP/2!");
let frame = DataFrame::new(1, data, false)
    .with_padding(10);

// Encode to wire format
let encoded = FrameCodec::encode_data_frame(&frame);

// Create a SETTINGS frame
let settings = SettingsBuilder::new()
    .initial_window_size(65535)
    .max_concurrent_streams(100)
    .build()?;

let settings_frame = SettingsFrame::new(settings);
let encoded_settings = FrameCodec::encode_settings_frame(&settings_frame);
```

### Flow Control Management

```rust
use vtest2::http::h2::flow_control::FlowControlWindow;

// Create flow control window
let mut window = FlowControlWindow::new();

// Check available capacity
if window.can_send(1000) {
    // Consume window
    let sent = window.consume(1000)?;
    // ... send data ...
}

// Receive WINDOW_UPDATE
window.increase(500)?;
```

### Stream State Management

```rust
use vtest2::http::h2::stream::{H2Stream, StreamState, StreamManager};

// Create stream manager
let mut manager = StreamManager::new(true); // true = client

// Create a stream
let stream_id = manager.create_stream()?;
let stream = manager.get_or_create_stream(stream_id)?;

// Transition states
stream.send_headers(false)?; // Idle → Open
stream.send_data(100, true)?; // Open → HalfClosedLocal

// Check state
assert_eq!(stream.state(), StreamState::HalfClosedLocal);
```

## Testing Patterns

### Testing Flow Control Violations

```rust
#[test]
fn test_flow_control_violation() {
    let mut window = FlowControlWindow::new();

    // Consume entire window
    window.consume(DEFAULT_INITIAL_WINDOW_SIZE as usize)?;

    // Try to send more data - should fail or return 0
    let result = window.consume(1000)?;
    assert_eq!(result, 0);
}
```

### Testing Invalid Frame Sequences

```rust
#[test]
fn test_invalid_frame_sequence() {
    let mut stream = H2Stream::new(1);

    // Try to send DATA without HEADERS
    let result = stream.send_data(100, false);
    assert!(result.is_err());
}
```

### Testing Large Body Transfers

```rust
#[test]
fn test_large_body_transfer() {
    let large_data = vec![0u8; 100_000]; // 100KB
    let max_frame_size = 16384;

    // Data should be split into multiple frames
    let chunks: Vec<_> = large_data.chunks(max_frame_size).collect();
    assert!(chunks.len() > 1);

    for (i, chunk) in chunks.iter().enumerate() {
        let is_last = i == chunks.len() - 1;
        let frame = DataFrame::new(1, Bytes::from(chunk.to_vec()), is_last);
        // Send frame...
    }
}
```

### Testing Concurrent Streams

```rust
#[test]
fn test_concurrent_streams() {
    let mut manager = StreamManager::new(true);
    manager.set_max_concurrent_streams(Some(100));

    // Create multiple streams
    let mut streams = vec![];
    for _ in 0..10 {
        let stream_id = manager.create_stream()?;
        streams.push(stream_id);
    }

    // Verify all streams are tracked
    assert_eq!(manager.active_stream_count(), 10);
}
```

## Protocol Constants

### Default Values

```rust
pub const DEFAULT_INITIAL_WINDOW_SIZE: u32 = 65535;
pub const DEFAULT_MAX_FRAME_SIZE: u32 = 16384;
pub const DEFAULT_HEADER_TABLE_SIZE: u32 = 4096;
pub const MAX_STREAM_ID: u32 = 0x7FFFFFFF;
```

### Connection Preface

Every HTTP/2 connection must begin with the connection preface:

```rust
pub const CONNECTION_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
```

### Frame Types

| Type | Value | Description |
|------|-------|-------------|
| DATA | 0x0 | Application data |
| HEADERS | 0x1 | Header block |
| PRIORITY | 0x2 | Stream priority |
| RST_STREAM | 0x3 | Stream termination |
| SETTINGS | 0x4 | Connection parameters |
| PUSH_PROMISE | 0x5 | Server push |
| PING | 0x6 | Connection liveness |
| GOAWAY | 0x7 | Connection termination |
| WINDOW_UPDATE | 0x8 | Flow control |
| CONTINUATION | 0x9 | Header continuation |

### Frame Flags

| Flag | Value | Applies To |
|------|-------|------------|
| END_STREAM | 0x1 | DATA, HEADERS |
| ACK | 0x1 | SETTINGS, PING |
| END_HEADERS | 0x4 | HEADERS, PUSH_PROMISE, CONTINUATION |
| PADDED | 0x8 | DATA, HEADERS, PUSH_PROMISE |
| PRIORITY | 0x20 | HEADERS |

### Error Codes

| Error | Value | Description |
|-------|-------|-------------|
| NO_ERROR | 0x0 | Graceful shutdown |
| PROTOCOL_ERROR | 0x1 | Protocol violation |
| INTERNAL_ERROR | 0x2 | Implementation error |
| FLOW_CONTROL_ERROR | 0x3 | Flow control violation |
| SETTINGS_TIMEOUT | 0x4 | SETTINGS ACK timeout |
| STREAM_CLOSED | 0x5 | Frame on closed stream |
| FRAME_SIZE_ERROR | 0x6 | Invalid frame size |
| REFUSED_STREAM | 0x7 | Stream refused |
| CANCEL | 0x8 | Stream cancelled |
| COMPRESSION_ERROR | 0x9 | HPACK error |
| CONNECT_ERROR | 0xa | TCP connection error |
| ENHANCE_YOUR_CALM | 0xb | Rate limiting |
| INADEQUATE_SECURITY | 0xc | TLS requirements not met |
| HTTP_1_1_REQUIRED | 0xd | HTTP/1.1 required |

### SETTINGS Parameters

| Parameter | ID | Default | Description |
|-----------|----|---------| ------------|
| HEADER_TABLE_SIZE | 0x1 | 4096 | HPACK dynamic table size |
| ENABLE_PUSH | 0x2 | 1 | Server push enabled |
| MAX_CONCURRENT_STREAMS | 0x3 | ∞ | Maximum concurrent streams |
| INITIAL_WINDOW_SIZE | 0x4 | 65535 | Initial flow control window |
| MAX_FRAME_SIZE | 0x5 | 16384 | Maximum frame payload size |
| MAX_HEADER_LIST_SIZE | 0x6 | ∞ | Maximum header list size |

## Stream States

HTTP/2 streams follow this state machine:

```
                         +--------+
                 send PP |        | recv PP
                ,--------|  idle  |--------.
               /         |        |         \
              v          +--------+          v
       +----------+          |           +----------+
       |          |          | send H /  |          |
,------| reserved |          | recv H    | reserved |------.
|      | (local)  |          |           | (remote) |      |
|      +----------+          v           +----------+      |
|          |             +--------+             |          |
|          |     recv ES |        | send ES     |          |
|   send H |     ,-------|  open  |-------.     | recv H   |
|          |    /        |        |        \    |          |
|          v   v         +--------+         v   v          |
|      +----------+          |           +----------+      |
|      |   half   |          |           |   half   |      |
|      |  closed  |          | send R /  |  closed  |      |
|      | (remote) |          | recv R    | (local)  |      |
|      +----------+          |           +----------+      |
|           |                |                 |           |
|           | send ES /      |       recv ES / |           |
|           | send R /       v        send R / |           |
|           | recv R     +--------+   recv R   |           |
| send R /  `----------->|        |<-----------'  send R / |
| recv R                 | closed |               recv R   |
`----------------------->|        |<----------------------'
                         +--------+

   send:   endpoint sends this frame
   recv:   endpoint receives this frame

   H:  HEADERS frame (with implied CONTINUATIONs)
   PP: PUSH_PROMISE frame (with implied CONTINUATIONs)
   ES: END_STREAM flag
   R:  RST_STREAM frame
```

## Integration with VTest2

The HTTP/2 implementation integrates with VTest2's existing architecture:

### Session Operations

HTTP/2 uses the same `SessionOps` trait as HTTP/1.1, allowing transparent use of:
- Plain TCP connections (`FdSessionOps`)
- TLS-encrypted connections (`TlsSessionOps`)

### TLS Integration

HTTP/2 requires TLS with ALPN negotiation:

```rust
// Server advertises h2
let config = TlsConfig::server()
    .alpn(&["h2", "http/1.1"])
    .build()?;

// Client requests h2
let config = TlsConfig::client()
    .alpn(&["h2"])
    .build()?;
```

## Performance Considerations

### Flow Control Tuning

- **Initial Window Size**: Default 65535 bytes. Increase for high-bandwidth connections.
- **Max Frame Size**: Default 16384 bytes (16KB). Can increase up to 16777215 (16MB - 1).
- **Max Concurrent Streams**: No default limit. Set based on server capacity.

### Memory Usage

- Each stream maintains its own flow control window
- Header compression uses a dynamic table (default 4KB)
- Larger windows enable better throughput but use more memory

### Recommendations

- Use initial window size of 1MB+ for high-bandwidth connections
- Limit concurrent streams based on available memory
- Enable header compression for repeated headers
- Use PUSH_PROMISE for predictable resources

## Testing Checklist

When testing HTTP/2 implementations, verify:

- [x] Connection preface exchange
- [x] SETTINGS frame exchange and ACK
- [x] Stream ID assignment (odd/even for client/server)
- [x] Flow control (connection and stream level)
- [x] WINDOW_UPDATE frame handling
- [x] Stream state transitions
- [x] Header compression/decompression
- [x] DATA frame handling
- [x] PING/PONG exchange
- [x] Graceful connection termination (GOAWAY)
- [x] Error handling (RST_STREAM, GOAWAY)
- [x] Frame size limits
- [x] Concurrent stream limits
- [x] Priority handling (optional)
- [x] Server push (optional)

## References

- [RFC 7540](https://tools.ietf.org/html/rfc7540) - HTTP/2 Specification
- [RFC 7541](https://tools.ietf.org/html/rfc7541) - HPACK Header Compression
- [RFC 8740](https://tools.ietf.org/html/rfc8740) - HTTP/2 over Unencrypted TCP (Informational)

## Future Enhancements

Potential improvements for the HTTP/2 implementation:

- [ ] Priority tree implementation (RFC 7540 Section 5.3)
- [ ] Server push support in client
- [ ] HTTP/2 Upgrade from HTTP/1.1
- [ ] Extended CONNECT support
- [ ] HTTP/3 (QUIC) support

## Troubleshooting

### Common Issues

**Problem**: Connection fails with PROTOCOL_ERROR
**Solution**: Verify connection preface is sent correctly

**Problem**: Flow control window exhausted
**Solution**: Send WINDOW_UPDATE frames when consuming data

**Problem**: Stream closed unexpectedly
**Solution**: Check stream state before sending frames

**Problem**: GOAWAY received immediately
**Solution**: Check ALPN negotiation, verify h2 is advertised

### Debug Tips

- Enable detailed logging for frame exchange
- Verify TLS ALPN negotiation succeeds
- Check SETTINGS frame parameters
- Monitor flow control window sizes
- Validate frame sizes against limits
