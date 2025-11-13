# HTTP Layer Implementation

This document describes the Rust implementation of the HTTP/1.1 layer for VTest2.

## Overview

The HTTP layer provides client and server functionality for testing HTTP/1.1 applications. It's a port of the C `vtc_http.c` module with idiomatic Rust patterns and strong type safety.

## Architecture

### Session Operations Pattern

The HTTP layer uses a **session operations abstraction pattern** that allows transparent switching between different transport layers (plain TCP, TLS, etc.) without changing the HTTP protocol handling code.

```rust
pub trait SessionOps {
    fn poll(&self, events: PollEvents, timeout: Option<Duration>) -> Result<bool>;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    fn write(&mut self, buf: &[u8]) -> Result<usize>;
    fn close(&mut self) -> Result<()>;
}
```

This design follows the C implementation's `struct sess_ops` pattern:
- `HttpSession<S>` wraps a `SessionOps` implementation
- Default implementation `FdSessionOps` provides plain TCP operations
- TLS support can be added by implementing `SessionOps` for TLS connections
- All HTTP I/O code works transparently with any transport

### Module Structure

```
src/http/
├── mod.rs           - Public API and error types
├── message.rs       - HTTP message types (Request, Response)
├── headers.rs       - Header collection with case-insensitive lookup
├── parser.rs        - HTTP request/response parsers
├── client.rs        - HTTP client (txreq, rxresp)
├── server.rs        - HTTP server (rxreq, txresp)
├── session.rs       - Session operations abstraction
└── chunked.rs       - Chunked transfer encoding
```

## Core Types

### HTTP Methods

```rust
pub enum Method {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Connect,
    Options,
    Trace,
    Patch,
}
```

### HTTP Versions

```rust
pub enum Version {
    Http10,   // HTTP/1.0
    Http11,   // HTTP/1.1
}
```

### HTTP Status

```rust
pub struct Status {
    code: u16,
}

impl Status {
    pub const OK: Status = Status { code: 200 };
    pub const NOT_FOUND: Status = Status { code: 404 };
    pub const INTERNAL_SERVER_ERROR: Status = Status { code: 500 };
    // ... more predefined statuses
}
```

### Headers

The `Headers` type provides case-insensitive header lookups and supports multiple values for the same header name:

```rust
let mut headers = Headers::new();
headers.insert("Content-Type", "text/html");
headers.insert("Set-Cookie", "a=1");
headers.insert("Set-Cookie", "b=2");

// Case-insensitive lookup
assert_eq!(headers.get("content-type"), Some("text/html"));

// Multiple values
let cookies = headers.get_all("Set-Cookie");
assert_eq!(cookies.len(), 2);
```

### HTTP Request

```rust
let request = HttpRequest::builder()
    .method(Method::Post)
    .uri("/api/data")
    .header("Content-Type", "application/json")
    .header("Authorization", "Bearer token")
    .body(b"{\"key\":\"value\"}".to_vec())
    .build();
```

### HTTP Response

```rust
let response = HttpResponse::builder()
    .status(Status::OK)
    .header("Content-Type", "application/json")
    .header("Content-Length", "13")
    .body(b"{\"ok\":true}".to_vec())
    .build();
```

## HTTP Client

The `HttpClient` provides methods for sending requests and receiving responses:

```rust
use vtest2::http::{HttpClient, HttpRequest, Method};
use vtest2::http::session::FdSessionOps;
use std::net::TcpStream;

// Connect to server
let stream = TcpStream::connect("127.0.0.1:8080")?;
let session = FdSessionOps::new(stream);
let mut client = HttpClient::new(session);

// Send request
let request = HttpRequest::builder()
    .method(Method::Get)
    .uri("/")
    .header("Host", "localhost")
    .build();

client.send_request(&request)?;

// Receive response
let response = client.receive_response()?;
assert_eq!(response.status().code(), 200);
```

### Convenience Methods

```rust
// Simple GET request
let response = client.get("/api/users")?;

// Simple POST request
let body = b"data".to_vec();
let response = client.post("/api/submit", body)?;
```

## HTTP Server

The `HttpServer` provides methods for receiving requests and sending responses:

```rust
use vtest2::http::{HttpServer, HttpResponse, Status};
use vtest2::http::session::FdSessionOps;
use std::net::TcpListener;

let listener = TcpListener::bind("127.0.0.1:8080")?;
let (stream, _) = listener.accept()?;
let session = FdSessionOps::new(stream);
let mut server = HttpServer::new(session);

// Receive request
let request = server.receive_request()?;
println!("Received {} {}", request.method(), request.uri());

// Send response
let response = HttpResponse::builder()
    .status(Status::OK)
    .header("Content-Type", "text/plain")
    .body(b"Hello, World!".to_vec())
    .build();

server.send_response(&response)?;
```

### Convenience Methods

```rust
// Send 200 OK
server.send_ok(b"Success")?;

// Send error
server.send_error(Status::NOT_FOUND, "Not Found")?;
```

## Body Handling

### Content-Length

Bodies with an explicit `Content-Length` header are read in full:

```rust
let request = HttpRequest::builder()
    .method(Method::Post)
    .uri("/upload")
    .header("Content-Length", "100")
    .body(vec![0u8; 100])
    .build();
```

### Chunked Transfer Encoding

The HTTP layer supports chunked transfer encoding for both reading and writing:

```rust
use vtest2::http::chunked::{encode_chunked_body, decode_chunked_body};

// Encode
let data = b"Hello World";
let chunked = encode_chunked_body(data, 5)?;
// Result: "5\r\nHello\r\n6\r\n World\r\n0\r\n\r\n"

// Decode
let decoded = decode_chunked_body(&chunked)?;
assert_eq!(decoded, data);
```

Server can send chunked responses:

```rust
let chunks: Vec<&[u8]> = vec![b"First", b"Second", b"Third"];
server.send_chunked_response(Status::OK, &headers, &chunks)?;
```

## HTTP Parsers

The HTTP layer includes incremental parsers for both requests and responses:

```rust
use vtest2::http::parser::{ResponseParser, RequestParser};

// Parse response incrementally
let mut parser = ResponseParser::new();

// Feed data as it arrives
let data1 = b"HTTP/1.1 200 OK\r\n";
assert!(parser.parse(data1)?.is_none()); // Need more data

let data2 = b"Content-Length: 5\r\n\r\nHello";
let response = parser.parse(data2)?.unwrap();

assert_eq!(response.status().code(), 200);
assert_eq!(response.body(), b"Hello");
```

## Error Handling

All HTTP operations return `Result<T, Error>`:

```rust
pub enum Error {
    Io(std::io::Error),
    Network(crate::net::Error),
    Parse(String),
    InvalidVersion(String),
    InvalidMethod(String),
    InvalidStatus(String),
    InvalidHeader(String),
    InvalidChunkSize(String),
    Incomplete,
    Timeout,
    ConnectionClosed,
    Protocol(String),
}
```

## Testing

### Unit Tests

Each module includes comprehensive unit tests. Run with:

```bash
cargo test --lib http::
```

The test suite includes:
- Message type conversions (Method, Version, Status)
- Header operations (insert, get, case-insensitivity, multi-value)
- Request/Response building and wire format
- HTTP parsing (request lines, status lines, headers, bodies)
- Chunked encoding/decoding
- Client/Server operations

### Integration Tests

End-to-end tests verify complete request/response cycles:

```bash
cargo test --test http_integration
```

Integration tests cover:
- Basic HTTP request/response cycle
- POST requests with bodies
- Multiple requests on the same connection (keep-alive)
- Large body handling
- Case-insensitive header lookups
- Error responses (404, etc.)

## Comparison with C Implementation

### Similarities

1. **Session Operations Pattern**: Both implementations use function pointers (C) / traits (Rust) for transport abstraction
2. **Command Structure**: VTC commands (txreq, rxresp, etc.) map to Rust methods
3. **Header Storage**: Case-insensitive lookups, multi-value support
4. **Chunked Encoding**: Full support for reading and writing
5. **Maximum Headers**: Both enforce `MAX_HDR` (64) limit

### Differences

1. **Type Safety**: Rust uses enums for Methods, Versions, and Status codes
2. **Builder Pattern**: Rust provides fluent builders for constructing messages
3. **Error Handling**: Rust uses `Result<T, E>` instead of C error codes
4. **Memory Safety**: Rust eliminates buffer overflows and use-after-free bugs
5. **String Handling**: Rust's `String` type vs C's null-terminated strings

## Performance Considerations

1. **Zero-Copy Where Possible**: Parsers work directly on input buffers
2. **Buffering**: Client and server maintain internal buffers to reduce syscalls
3. **Incremental Parsing**: Parsers support feeding data incrementally
4. **Allocation**: Headers and bodies are allocated once and reused when possible

## Future Enhancements

1. **TLS Support**: Implement `SessionOps` for TLS connections
2. **HTTP/2**: Port `vtc_http2.c` functionality
3. **Connection Pooling**: For client-side connection reuse
4. **Streaming Bodies**: Support for large bodies that don't fit in memory
5. **Compression**: gzip/deflate support (already present in C version)

## Examples

### Simple HTTP Server

```rust
use vtest2::http::{HttpServer, Status};
use vtest2::http::session::FdSessionOps;
use std::net::TcpListener;

let listener = TcpListener::bind("127.0.0.1:8080")?;

loop {
    let (stream, _) = listener.accept()?;
    std::thread::spawn(move || {
        let session = FdSessionOps::new(stream);
        let mut server = HttpServer::new(session);

        let request = server.receive_request()?;

        match request.uri() {
            "/" => server.send_ok(b"Welcome!"),
            "/hello" => server.send_ok(b"Hello, World!"),
            _ => server.send_error(Status::NOT_FOUND, "Not Found"),
        }
    });
}
```

### HTTP Client with Retries

```rust
use vtest2::http::{HttpClient, Error};
use vtest2::http::session::FdSessionOps;
use std::net::TcpStream;

fn fetch_with_retry(url: &str, retries: usize) -> Result<Vec<u8>, Error> {
    for attempt in 0..retries {
        match TcpStream::connect("127.0.0.1:8080") {
            Ok(stream) => {
                let session = FdSessionOps::new(stream);
                let mut client = HttpClient::new(session);

                match client.get(url) {
                    Ok(response) if response.status().is_success() => {
                        return Ok(response.body().to_vec());
                    }
                    Ok(response) => {
                        eprintln!("HTTP error: {}", response.status());
                    }
                    Err(e) => {
                        eprintln!("Attempt {}/{} failed: {}", attempt + 1, retries, e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    Err(Error::Timeout)
}
```

## References

- C Implementation: `src/vtc_http.c`, `src/vtc_http.h`
- RFC 7230: HTTP/1.1 Message Syntax and Routing
- RFC 7231: HTTP/1.1 Semantics and Content
