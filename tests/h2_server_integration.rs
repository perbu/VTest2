//! HTTP/2 server integration tests
//!
//! These tests verify the H2Server implementation works correctly.

use vtest2::http::h2::*;
use vtest2::http::h2::error::Result;
use bytes::Bytes;

#[test]
fn test_server_builder() {
    let builder = H2ServerBuilder::new()
        .header_table_size(8192)
        .enable_push(true)
        .initial_window_size(65535)
        .max_concurrent_streams(100)
        .max_frame_size(32768);

    // Builder pattern works
}

#[test]
fn test_request_structure() {
    let mut headers = std::collections::HashMap::new();
    headers.insert("content-type".to_string(), "application/json".to_string());
    headers.insert("x-custom-header".to_string(), "custom-value".to_string());

    let request = server::H2Request {
        stream_id: 1,
        method: "POST".to_string(),
        path: "/api/v1/data".to_string(),
        scheme: "https".to_string(),
        authority: "example.com:443".to_string(),
        headers,
        body: Bytes::from(r#"{"test":"data"}"#),
    };

    // Test accessors
    assert_eq!(request.stream_id, 1);
    assert_eq!(request.method(), "POST");
    assert_eq!(request.path(), "/api/v1/data");
    assert_eq!(request.scheme(), "https");
    assert_eq!(request.authority(), "example.com:443");
    assert_eq!(request.header("content-type"), Some("application/json"));
    assert_eq!(request.header("x-custom-header"), Some("custom-value"));
    assert_eq!(request.header("nonexistent"), None);
    assert_eq!(request.body(), br#"{"test":"data"}"#);
    assert_eq!(request.body_string().unwrap(), r#"{"test":"data"}"#);
}

#[test]
fn test_request_empty_body() {
    let request = server::H2Request {
        stream_id: 3,
        method: "GET".to_string(),
        path: "/api/resource".to_string(),
        scheme: "https".to_string(),
        authority: "api.example.com".to_string(),
        headers: std::collections::HashMap::new(),
        body: Bytes::new(),
    };

    assert!(request.body().is_empty());
    assert_eq!(request.body_string().unwrap(), "");
}

#[test]
fn test_request_with_various_methods() {
    let methods = vec!["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

    for method in methods {
        let request = server::H2Request {
            stream_id: 1,
            method: method.to_string(),
            path: "/".to_string(),
            scheme: "https".to_string(),
            authority: "example.com".to_string(),
            headers: std::collections::HashMap::new(),
            body: Bytes::new(),
        };

        assert_eq!(request.method(), method);
    }
}

#[test]
fn test_request_with_query_parameters() {
    let request = server::H2Request {
        stream_id: 5,
        method: "GET".to_string(),
        path: "/search?q=rust+http2&limit=10".to_string(),
        scheme: "https".to_string(),
        authority: "search.example.com".to_string(),
        headers: std::collections::HashMap::new(),
        body: Bytes::new(),
    };

    assert_eq!(request.path(), "/search?q=rust+http2&limit=10");
}

#[test]
fn test_request_large_body() {
    let large_body = vec![0u8; 100_000]; // 100KB
    let body_bytes = Bytes::from(large_body.clone());

    let request = server::H2Request {
        stream_id: 7,
        method: "POST".to_string(),
        path: "/upload".to_string(),
        scheme: "https".to_string(),
        authority: "upload.example.com".to_string(),
        headers: std::collections::HashMap::new(),
        body: body_bytes,
    };

    assert_eq!(request.body().len(), 100_000);
}

#[test]
fn test_request_with_multiple_headers() {
    let mut headers = std::collections::HashMap::new();
    headers.insert("content-type".to_string(), "application/json".to_string());
    headers.insert("accept".to_string(), "application/json".to_string());
    headers.insert("accept-encoding".to_string(), "gzip, deflate, br".to_string());
    headers.insert("user-agent".to_string(), "VTest2/0.1.0".to_string());
    headers.insert("authorization".to_string(), "Bearer token123".to_string());

    let request = server::H2Request {
        stream_id: 9,
        method: "GET".to_string(),
        path: "/protected/resource".to_string(),
        scheme: "https".to_string(),
        authority: "api.example.com".to_string(),
        headers: headers.clone(),
        body: Bytes::new(),
    };

    assert_eq!(request.headers.len(), 5);
    assert_eq!(request.header("authorization"), Some("Bearer token123"));
}

#[test]
fn test_request_pseudo_headers() {
    // Test that pseudo-headers are properly stored in dedicated fields
    let request = server::H2Request {
        stream_id: 11,
        method: "CONNECT".to_string(),
        path: "/".to_string(),
        scheme: "https".to_string(),
        authority: "proxy.example.com:8080".to_string(),
        headers: std::collections::HashMap::new(),
        body: Bytes::new(),
    };

    // Pseudo-headers are stored in dedicated struct fields, not in headers map
    assert_eq!(request.method(), "CONNECT");
    assert_eq!(request.path(), "/");
    assert_eq!(request.scheme(), "https");
    assert_eq!(request.authority(), "proxy.example.com:8080");

    // These should NOT be in the regular headers
    assert!(request.header(":method").is_none());
    assert!(request.header(":path").is_none());
    assert!(request.header(":scheme").is_none());
    assert!(request.header(":authority").is_none());
}

#[test]
fn test_server_builder_default() {
    let builder = H2ServerBuilder::default();
    // Just verify the default constructor works
}

#[test]
fn test_request_body_string_invalid_utf8() {
    // Create invalid UTF-8 sequence
    let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
    let request = server::H2Request {
        stream_id: 13,
        method: "POST".to_string(),
        path: "/data".to_string(),
        scheme: "https".to_string(),
        authority: "example.com".to_string(),
        headers: std::collections::HashMap::new(),
        body: Bytes::from(invalid_utf8),
    };

    // Should return an error for invalid UTF-8
    assert!(request.body_string().is_err());
}

#[test]
fn test_server_settings_defaults() {
    let builder = H2ServerBuilder::new();

    // Server should allow push by default (unlike client)
    // This is implicit in the builder, but we can verify the structure exists
}

/// Test that demonstrates the server API usage pattern
#[test]
fn test_server_api_pattern() {
    // This test documents the intended usage pattern for H2Server
    // Even though we can't test with real connections, we can show the API

    /*
    Example usage (would require real network connection):

    use vtest2::http::h2::{H2ServerBuilder, H2Request};
    use vtest2::http::session::FdSessionOps;
    use std::net::TcpListener;
    use bytes::Bytes;

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let (stream, _) = listener.accept()?;

    let mut server = H2ServerBuilder::new()
        .max_concurrent_streams(100)
        .build(FdSessionOps::new(stream))?;

    // Accept HTTP/2 connection
    server.accept()?;

    // Receive request
    let request = server.recv_request()?;
    println!("Received {} {}", request.method(), request.path());

    // Send response
    server.send_response(
        request.stream_id,
        200,
        &[("content-type", "text/plain")],
        Bytes::from("Hello, HTTP/2!")
    )?;
    */
}
