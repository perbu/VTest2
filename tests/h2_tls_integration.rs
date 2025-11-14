//! HTTP/2 over TLS integration tests
//!
//! These tests verify end-to-end HTTP/2 functionality over TLS including:
//! - ALPN negotiation for "h2" protocol
//! - Connection preface and settings exchange
//! - Request/response cycles
//! - Multiple concurrent requests
//! - Custom headers
//! - PING/PONG exchange
//! - RST_STREAM error handling
//! - GOAWAY connection termination
//! - TLS version and ALPN verification

use vtest2::http::h2::*;
use vtest2::http::tls::{TlsConfig, TlsVersion};
use bytes::Bytes;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

/// Helper to create a test server that accepts one connection
fn spawn_test_server<F>(handler: F) -> u16
where
    F: FnOnce(H2Server<vtest2::http::tls::TlsSessionOps>) + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    thread::spawn(move || {
        let (tcp_stream, _) = listener.accept().unwrap();

        // Create TLS config with ALPN for h2
        let tls_config = TlsConfig::server()
            .version(TlsVersion::Tls12)
            .alpn(&["h2"])
            .unwrap()
            .build()
            .unwrap();

        // Accept TLS connection
        let tls_session = tls_config.accept(tcp_stream).unwrap();

        // Create HTTP/2 server and accept connection
        let mut server = H2ServerBuilder::new().build(tls_session).unwrap();
        server.accept().unwrap();

        // Run handler
        handler(server);
    });

    // Give server time to start and accept TLS
    thread::sleep(Duration::from_millis(200));

    port
}

#[test]
fn test_h2_tls_connect_only() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    thread::spawn(move || {
        let (tcp_stream, _) = listener.accept().unwrap();

        let tls_config = TlsConfig::server()
            .version(TlsVersion::Tls12)
            .alpn(&["h2"])
            .unwrap()
            .build()
            .unwrap();

        let tls_session = tls_config.accept(tcp_stream).unwrap();
        let mut server = H2ServerBuilder::new().build(tls_session).unwrap();

        // Just do the accept handshake
        let result = server.accept();
        println!("Server accept result: {:?}", result);

        // Keep connection alive briefly
        thread::sleep(Duration::from_millis(500));
    });

    thread::sleep(Duration::from_millis(100));

    // Client side
    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls12)
        .servername("localhost")
        .alpn(&["h2"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
    let tls_session = tls_config.connect(tcp_stream).unwrap();

    let mut client = H2ClientBuilder::new().build(tls_session).unwrap();

    let result = client.connect();
    println!("Client connect result: {:?}", result);
    assert!(result.is_ok());
}

#[test]
fn test_h2_tls_simple_get() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    // Server thread
    thread::spawn(move || {
        let (tcp_stream, _) = listener.accept().unwrap();

        let tls_config = TlsConfig::server()
            .version(TlsVersion::Tls12)
            .alpn(&["h2"])
            .unwrap()
            .build()
            .unwrap();

        let tls_session = tls_config.accept(tcp_stream).unwrap();
        let mut server = H2ServerBuilder::new().build(tls_session).unwrap();

        // Accept HTTP/2 connection
        server.accept().unwrap();

        // Receive request
        let request = server.receive_request().unwrap();
        assert_eq!(request.method(), "GET");
        assert_eq!(request.path(), "/");

        // Send response
        server
            .send_response(request.stream_id, 200, &[], Bytes::from("Hello, HTTP/2!"))
            .unwrap();
    });

    thread::sleep(Duration::from_millis(100));

    // Client side
    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls12)
        .servername("localhost")
        .alpn(&["h2"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
    let tls_session = tls_config.connect(tcp_stream).unwrap();

    let mut client = H2ClientBuilder::new().build(tls_session).unwrap();

    // Connect and exchange settings
    client.connect().unwrap();

    // Send GET request
    let response = client.get("/").unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.body(), b"Hello, HTTP/2!");
}

#[test]
fn test_h2_tls_post_with_body() {
    let port = spawn_test_server(|mut server| {
        let request = server.receive_request().unwrap();
        assert_eq!(request.method(), "POST");
        assert_eq!(request.path(), "/api/data");
        assert_eq!(request.body(), b"{\"key\":\"value\"}");

        server
            .send_response(
                request.stream_id,
                201,
                &[("content-type", "application/json")],
                Bytes::from("{\"status\":\"created\"}"),
            )
            .unwrap();
    });

    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls12)
        .servername("localhost")
        .alpn(&["h2"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
    let tls_session = tls_config.connect(tcp_stream).unwrap();

    let mut client = H2ClientBuilder::new().build(tls_session).unwrap();
    client.connect().unwrap();

    let response = client
        .post(
            "/api/data",
            &[("content-type", "application/json")],
            Bytes::from("{\"key\":\"value\"}"),
        )
        .unwrap();

    assert_eq!(response.status(), 201);
    assert_eq!(response.header("content-type"), Some("application/json"));
    assert_eq!(response.body(), b"{\"status\":\"created\"}");
}

#[test]
fn test_h2_tls_multiple_requests() {
    let port = spawn_test_server(|mut server| {

        // First request
        let req1 = server.receive_request().unwrap();
        assert_eq!(req1.path(), "/first");
        server
            .send_response(req1.stream_id, 200, &[], Bytes::from("Response 1"))
            .unwrap();

        // Second request
        let req2 = server.receive_request().unwrap();
        assert_eq!(req2.path(), "/second");
        server
            .send_response(req2.stream_id, 200, &[], Bytes::from("Response 2"))
            .unwrap();

        // Third request
        let req3 = server.receive_request().unwrap();
        assert_eq!(req3.path(), "/third");
        server
            .send_response(req3.stream_id, 200, &[], Bytes::from("Response 3"))
            .unwrap();
    });

    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls12)
        .servername("localhost")
        .alpn(&["h2"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
    let tls_session = tls_config.connect(tcp_stream).unwrap();

    let mut client = H2ClientBuilder::new().build(tls_session).unwrap();
    client.connect().unwrap();

    // Send multiple requests
    let resp1 = client.get("/first").unwrap();
    assert_eq!(resp1.body(), b"Response 1");

    let resp2 = client.get("/second").unwrap();
    assert_eq!(resp2.body(), b"Response 2");

    let resp3 = client.get("/third").unwrap();
    assert_eq!(resp3.body(), b"Response 3");
}

#[test]
fn test_h2_tls_custom_headers() {
    let port = spawn_test_server(|mut server| {

        let request = server.receive_request().unwrap();
        assert_eq!(request.header("x-custom-header"), Some("custom-value"));
        assert_eq!(request.header("x-test-id"), Some("12345"));

        server
            .send_response(
                request.stream_id,
                200,
                &[
                    ("x-response-header", "response-value"),
                    ("x-server-version", "1.0"),
                ],
                Bytes::from("OK"),
            )
            .unwrap();
    });

    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls12)
        .servername("localhost")
        .alpn(&["h2"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
    let tls_session = tls_config.connect(tcp_stream).unwrap();

    let mut client = H2ClientBuilder::new().build(tls_session).unwrap();
    client.connect().unwrap();

    let response = client
        .request(
            "GET",
            "/test",
            &[
                ("x-custom-header", "custom-value"),
                ("x-test-id", "12345"),
            ],
            Bytes::new(),
        )
        .unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(
        response.header("x-response-header"),
        Some("response-value")
    );
    assert_eq!(response.header("x-server-version"), Some("1.0"));
}

#[test]
fn test_h2_tls_ping_pong() {
    let port = spawn_test_server(|mut server| {

        // Receive PING
        let (frame_type, _flags, _stream_id, payload) = server.recv_frame().unwrap();
        assert_eq!(frame_type, FrameType::Ping);
        assert_eq!(payload.len(), 8);

        // PING is automatically responded to, so we just receive the request
        let request = server.receive_request().unwrap();
        server
            .send_response(request.stream_id, 200, &[], Bytes::from("Pong"))
            .unwrap();
    });

    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls12)
        .servername("localhost")
        .alpn(&["h2"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
    let tls_session = tls_config.connect(tcp_stream).unwrap();

    let mut client = H2ClientBuilder::new().build(tls_session).unwrap();
    client.connect().unwrap();

    // Send PING
    let ping_data = [1, 2, 3, 4, 5, 6, 7, 8];
    client.send_ping(ping_data).unwrap();

    // Send a regular request
    let response = client.get("/").unwrap();
    assert_eq!(response.status(), 200);
}

#[test]
fn test_h2_tls_settings_exchange() {
    let port = spawn_test_server(|mut server| {

        // Check remote settings
        let remote_settings = server.remote_settings();
        assert!(remote_settings.header_table_size.is_some());

        let request = server.receive_request().unwrap();
        server
            .send_response(request.stream_id, 200, &[], Bytes::from("OK"))
            .unwrap();
    });

    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls12)
        .servername("localhost")
        .alpn(&["h2"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
    let tls_session = tls_config.connect(tcp_stream).unwrap();

    let mut client = H2ClientBuilder::new()
        .header_table_size(8192)
        .initial_window_size(65535)
        .build(tls_session)
        .unwrap();

    client.connect().unwrap();

    // Check that settings were exchanged
    let remote_settings = client.remote_settings();
    assert!(remote_settings.header_table_size.is_some());

    let response = client.get("/").unwrap();
    assert_eq!(response.status(), 200);
}

#[test]
fn test_h2_tls_rst_stream() {
    let port = spawn_test_server(|mut server| {

        let request = server.receive_request().unwrap();

        // Send RST_STREAM instead of response
        server
            .send_rst_stream(request.stream_id, ErrorCode::InternalError)
            .unwrap();
    });

    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls12)
        .servername("localhost")
        .alpn(&["h2"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
    let tls_session = tls_config.connect(tcp_stream).unwrap();

    let mut client = H2ClientBuilder::new().build(tls_session).unwrap();
    client.connect().unwrap();

    // This should receive RST_STREAM error
    let result = client.get("/");
    assert!(result.is_err());
}

#[test]
fn test_h2_tls_goaway() {
    let port = spawn_test_server(|mut server| {

        let request = server.receive_request().unwrap();
        server
            .send_response(request.stream_id, 200, &[], Bytes::from("OK"))
            .unwrap();

        // Send GOAWAY
        server
            .send_goaway(request.stream_id, ErrorCode::NoError, "shutdown")
            .unwrap();
    });

    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls12)
        .servername("localhost")
        .alpn(&["h2"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
    let tls_session = tls_config.connect(tcp_stream).unwrap();

    let mut client = H2ClientBuilder::new().build(tls_session).unwrap();
    client.connect().unwrap();

    let response = client.get("/").unwrap();
    assert_eq!(response.status(), 200);

    // Second request should fail due to GOAWAY
    let result = client.get("/second");
    assert!(result.is_err());
}

#[test]
fn test_h2_tls_alpn_negotiation() {
    let port = spawn_test_server(|mut _server| {
        // Just verify the connection was established with ALPN
        // The ALPN negotiation happens during TLS handshake
    });

    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls12)
        .servername("localhost")
        .alpn(&["h2", "http/1.1"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
    let tls_session = tls_config.connect(tcp_stream).unwrap();

    // Verify ALPN negotiated to h2
    // Note: You would need to expose the negotiated protocol from TlsSessionOps
    // For now, just verify the connection succeeds

    let _client = H2ClientBuilder::new().build(tls_session).unwrap();
}

#[test]
fn test_h2_tls_version_verification() {
    let port = spawn_test_server(|mut _server| {
        // Server accepts with TLS 1.2
    });

    // Connect with TLS 1.2
    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls12)
        .servername("localhost")
        .alpn(&["h2"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
    let result = tls_config.connect(tcp_stream);
    assert!(result.is_ok());
}

#[test]
fn test_h2_tls_window_update() {
    let port = spawn_test_server(|mut server| {

        // Send WINDOW_UPDATE
        server.send_window_update(0, 1024).unwrap();

        let request = server.receive_request().unwrap();
        server
            .send_response(request.stream_id, 200, &[], Bytes::from("OK"))
            .unwrap();
    });

    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls12)
        .servername("localhost")
        .alpn(&["h2"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
    let tls_session = tls_config.connect(tcp_stream).unwrap();

    let mut client = H2ClientBuilder::new().build(tls_session).unwrap();
    client.connect().unwrap();

    let response = client.get("/").unwrap();
    assert_eq!(response.status(), 200);
}

#[test]
fn test_h2_tls_large_body() {
    // Test with body larger than initial window size
    let large_body = Bytes::from(vec![b'X'; 100_000]); // 100KB
    let large_body_clone = large_body.clone();

    let port = spawn_test_server(move |mut server| {

        let request = server.receive_request().unwrap();
        assert_eq!(request.body().len(), large_body_clone.len());

        server
            .send_response(request.stream_id, 200, &[], Bytes::from("Received"))
            .unwrap();
    });

    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls12)
        .servername("localhost")
        .alpn(&["h2"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
    let tls_session = tls_config.connect(tcp_stream).unwrap();

    let mut client = H2ClientBuilder::new().build(tls_session).unwrap();
    client.connect().unwrap();

    let response = client.post("/upload", &[], large_body).unwrap();
    assert_eq!(response.status(), 200);
}
