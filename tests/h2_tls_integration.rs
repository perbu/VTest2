//! HTTP/2 over TLS integration tests
//!
//! Comprehensive tests for HTTP/2 protocol implementation over TLS with ALPN.
//! These tests demonstrate the full stack working together and test
//! edge cases and protocol violations - the core purpose of VTest2.

use vtest2::http::h2::{H2Client, H2Server, ErrorCode};
use vtest2::http::tls::{TlsConfig, TlsVersion, TlsSessionOps};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;
use bytes::Bytes;

#[test]
fn test_h2_over_tls_simple_get() {
    // Create listener on random port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let server_addr = listener.local_addr().unwrap();

    // Spawn server thread
    let server_handle = thread::spawn(move || {
        let tls_config = TlsConfig::server()
            .version(TlsVersion::Tls13)
            .alpn(&["h2", "http/1.1"])
            .unwrap()
            .build()
            .unwrap();

        let (tcp_stream, _) = listener.accept().unwrap();
        let tls_session = tls_config.accept(tcp_stream).unwrap();

        let mut server: H2Server<TlsSessionOps> = H2Server::new(tls_session).unwrap();
        server.accept().unwrap();

        let request = server.recv_request().unwrap();
        assert_eq!(request.method(), "GET");
        assert_eq!(request.path(), "/");

        server
            .send_response(request.stream_id, 200, &[], Bytes::from("Hello, HTTP/2!"))
            .unwrap();
    });

    // Give server time to start
    thread::sleep(Duration::from_millis(100));

    // Client
    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls13)
        .alpn(&["h2"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(server_addr).unwrap();
    let tls_session = tls_config.connect(tcp_stream).unwrap();

    let mut client: H2Client<TlsSessionOps> = H2Client::new(tls_session).unwrap();
    client.connect().unwrap();

    let response = client.get("/").unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.body_string().unwrap(), "Hello, HTTP/2!");

    server_handle.join().unwrap();
}

#[test]
fn test_h2_over_tls_post_with_body() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let server_addr = listener.local_addr().unwrap();

    let server_handle = thread::spawn(move || {
        let tls_config = TlsConfig::server()
            .version(TlsVersion::Tls13)
            .alpn(&["h2"])
            .unwrap()
            .build()
            .unwrap();

        let (tcp_stream, _) = listener.accept().unwrap();
        let tls_session = tls_config.accept(tcp_stream).unwrap();

        let mut server: H2Server<TlsSessionOps> = H2Server::new(tls_session).unwrap();
        server.accept().unwrap();

        let request = server.recv_request().unwrap();
        assert_eq!(request.method(), "POST");
        assert_eq!(request.path(), "/api/data");
        assert_eq!(request.body_string().unwrap(), "test data");

        server
            .send_response(request.stream_id, 201, &[("content-type", "text/plain")], Bytes::from("Created"))
            .unwrap();
    });

    thread::sleep(Duration::from_millis(100));

    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls13)
        .alpn(&["h2"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(server_addr).unwrap();
    let tls_session = tls_config.connect(tcp_stream).unwrap();

    let mut client: H2Client<TlsSessionOps> = H2Client::new(tls_session).unwrap();
    client.connect().unwrap();

    let response = client
        .post("/api/data", &[], Bytes::from("test data"))
        .unwrap();
    assert_eq!(response.status(), 201);
    assert_eq!(response.header("content-type"), Some("text/plain"));
    assert_eq!(response.body_string().unwrap(), "Created");

    server_handle.join().unwrap();
}

#[test]
fn test_h2_over_tls_alpn_negotiation() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let server_addr = listener.local_addr().unwrap();

    let server_handle = thread::spawn(move || {
        let tls_config = TlsConfig::server()
            .version(TlsVersion::Tls13)
            .alpn(&["h2", "http/1.1"])
            .unwrap()
            .build()
            .unwrap();

        let (tcp_stream, _) = listener.accept().unwrap();
        let tls_session = tls_config.accept(tcp_stream).unwrap();

        // Check ALPN before creating server
        let alpn = tls_session.vars().alpn.clone();
        assert_eq!(alpn.as_deref(), Some("h2"));

        let mut server: H2Server<TlsSessionOps> = H2Server::new(tls_session).unwrap();

        // Check TLS vars from the server session
        let tls_vars = server.session().get_ref().vars();
        assert_eq!(tls_vars.alpn.as_deref(), Some("h2"));
        assert_eq!(tls_vars.version, "TLSv1.3");
        assert!(!tls_vars.failed);

        server.accept().unwrap();
        let request = server.recv_request().unwrap();
        server.send_response(request.stream_id, 200, &[], Bytes::from("OK")).unwrap();
    });

    thread::sleep(Duration::from_millis(100));

    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls13)
        .alpn(&["h2"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(server_addr).unwrap();
    let tls_session = tls_config.connect(tcp_stream).unwrap();

    let mut client: H2Client<TlsSessionOps> = H2Client::new(tls_session).unwrap();

    // Check client ALPN
    let tls_vars = client.session().get_ref().vars();
    assert_eq!(tls_vars.alpn.as_deref(), Some("h2"));

    client.connect().unwrap();
    let response = client.get("/").unwrap();
    assert_eq!(response.status(), 200);

    server_handle.join().unwrap();
}

#[test]
fn test_h2_multiple_requests() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let server_addr = listener.local_addr().unwrap();

    let server_handle = thread::spawn(move || {
        let tls_config = TlsConfig::server()
            .version(TlsVersion::Tls13)
            .alpn(&["h2"])
            .unwrap()
            .build()
            .unwrap();

        let (tcp_stream, _) = listener.accept().unwrap();
        let tls_session = tls_config.accept(tcp_stream).unwrap();

        let mut server: H2Server<TlsSessionOps> = H2Server::new(tls_session).unwrap();
        server.accept().unwrap();

        // Handle multiple requests
        for i in 0..3 {
            let request = server.recv_request().unwrap();
            let body = format!("Response {}", i);
            server
                .send_response(request.stream_id, 200, &[], Bytes::from(body))
                .unwrap();
        }
    });

    thread::sleep(Duration::from_millis(100));

    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls13)
        .alpn(&["h2"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(server_addr).unwrap();
    let tls_session = tls_config.connect(tcp_stream).unwrap();

    let mut client: H2Client<TlsSessionOps> = H2Client::new(tls_session).unwrap();
    client.connect().unwrap();

    // Send multiple requests (sequential)
    for _ in 0..3 {
        let response = client.get("/").unwrap();
        assert_eq!(response.status(), 200);
    }

    server_handle.join().unwrap();
}

#[test]
fn test_h2_settings_exchange() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let server_addr = listener.local_addr().unwrap();

    let server_handle = thread::spawn(move || {
        let tls_config = TlsConfig::server()
            .version(TlsVersion::Tls13)
            .alpn(&["h2"])
            .unwrap()
            .build()
            .unwrap();

        let (tcp_stream, _) = listener.accept().unwrap();
        let tls_session = tls_config.accept(tcp_stream).unwrap();

        let mut server: H2Server<TlsSessionOps> = H2Server::new(tls_session).unwrap();
        server.accept().unwrap();

        // Check that settings were exchanged
        assert!(server.local_settings().initial_window_size.is_some());
        assert!(server.remote_settings().initial_window_size.is_some());

        let request = server.recv_request().unwrap();
        server.send_response(request.stream_id, 200, &[], Bytes::from("OK")).unwrap();
    });

    thread::sleep(Duration::from_millis(100));

    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls13)
        .alpn(&["h2"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(server_addr).unwrap();
    let tls_session = tls_config.connect(tcp_stream).unwrap();

    let mut client: H2Client<TlsSessionOps> = H2Client::new(tls_session).unwrap();
    client.connect().unwrap();

    // Check settings were received
    assert!(client.local_settings().initial_window_size.is_some());
    assert!(client.remote_settings().initial_window_size.is_some());

    let response = client.get("/").unwrap();
    assert_eq!(response.status(), 200);

    server_handle.join().unwrap();
}

#[test]
fn test_h2_custom_headers() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let server_addr = listener.local_addr().unwrap();

    let server_handle = thread::spawn(move || {
        let tls_config = TlsConfig::server()
            .version(TlsVersion::Tls13)
            .alpn(&["h2"])
            .unwrap()
            .build()
            .unwrap();

        let (tcp_stream, _) = listener.accept().unwrap();
        let tls_session = tls_config.accept(tcp_stream).unwrap();

        let mut server: H2Server<TlsSessionOps> = H2Server::new(tls_session).unwrap();
        server.accept().unwrap();

        let request = server.recv_request().unwrap();
        assert_eq!(request.header("x-custom-header"), Some("custom-value"));
        assert_eq!(request.header("user-agent"), Some("test-client"));

        server
            .send_response(
                request.stream_id,
                200,
                &[
                    ("x-server-header", "server-value"),
                    ("content-type", "application/json"),
                ],
                Bytes::from("{}"),
            )
            .unwrap();
    });

    thread::sleep(Duration::from_millis(100));

    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls13)
        .alpn(&["h2"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(server_addr).unwrap();
    let tls_session = tls_config.connect(tcp_stream).unwrap();

    let mut client: H2Client<TlsSessionOps> = H2Client::new(tls_session).unwrap();
    client.connect().unwrap();

    let response = client
        .request(
            "GET",
            "/",
            &[("x-custom-header", "custom-value"), ("user-agent", "test-client")],
            Bytes::new(),
        )
        .unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(response.header("x-server-header"), Some("server-value"));
    assert_eq!(response.header("content-type"), Some("application/json"));

    server_handle.join().unwrap();
}

#[test]
fn test_h2_ping_pong() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let server_addr = listener.local_addr().unwrap();

    let server_handle = thread::spawn(move || {
        let tls_config = TlsConfig::server()
            .version(TlsVersion::Tls13)
            .alpn(&["h2"])
            .unwrap()
            .build()
            .unwrap();

        let (tcp_stream, _) = listener.accept().unwrap();
        let tls_session = tls_config.accept(tcp_stream).unwrap();

        let mut server: H2Server<TlsSessionOps> = H2Server::new(tls_session).unwrap();
        server.accept().unwrap();

        // Server sends PING
        server.send_ping([1, 2, 3, 4, 5, 6, 7, 8]).unwrap();

        let request = server.recv_request().unwrap();
        server.send_response(request.stream_id, 200, &[], Bytes::from("OK")).unwrap();
    });

    thread::sleep(Duration::from_millis(100));

    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls13)
        .alpn(&["h2"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(server_addr).unwrap();
    let tls_session = tls_config.connect(tcp_stream).unwrap();

    let mut client: H2Client<TlsSessionOps> = H2Client::new(tls_session).unwrap();
    client.connect().unwrap();

    // Client sends PING
    client.send_ping([8, 7, 6, 5, 4, 3, 2, 1]).unwrap();

    let response = client.get("/").unwrap();
    assert_eq!(response.status(), 200);

    server_handle.join().unwrap();
}

#[test]
fn test_h2_rst_stream() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let server_addr = listener.local_addr().unwrap();

    let server_handle = thread::spawn(move || {
        let tls_config = TlsConfig::server()
            .version(TlsVersion::Tls13)
            .alpn(&["h2"])
            .unwrap()
            .build()
            .unwrap();

        let (tcp_stream, _) = listener.accept().unwrap();
        let tls_session = tls_config.accept(tcp_stream).unwrap();

        let mut server: H2Server<TlsSessionOps> = H2Server::new(tls_session).unwrap();
        server.accept().unwrap();

        let request = server.recv_request().unwrap();

        // Send RST_STREAM instead of response
        server.send_rst_stream(request.stream_id, ErrorCode::Cancel).unwrap();
    });

    thread::sleep(Duration::from_millis(100));

    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls13)
        .alpn(&["h2"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(server_addr).unwrap();
    let tls_session = tls_config.connect(tcp_stream).unwrap();

    let mut client: H2Client<TlsSessionOps> = H2Client::new(tls_session).unwrap();
    client.connect().unwrap();

    let result = client.get("/");
    // Should receive error due to RST_STREAM
    assert!(result.is_err());

    server_handle.join().unwrap();
}

#[test]
fn test_h2_goaway() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let server_addr = listener.local_addr().unwrap();

    let server_handle = thread::spawn(move || {
        let tls_config = TlsConfig::server()
            .version(TlsVersion::Tls13)
            .alpn(&["h2"])
            .unwrap()
            .build()
            .unwrap();

        let (tcp_stream, _) = listener.accept().unwrap();
        let tls_session = tls_config.accept(tcp_stream).unwrap();

        let mut server: H2Server<TlsSessionOps> = H2Server::new(tls_session).unwrap();
        server.accept().unwrap();

        let request = server.recv_request().unwrap();
        server.send_response(request.stream_id, 200, &[], Bytes::from("OK")).unwrap();

        // Send GOAWAY
        server.send_goaway(request.stream_id, ErrorCode::NoError, "Shutting down").unwrap();
    });

    thread::sleep(Duration::from_millis(100));

    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls13)
        .alpn(&["h2"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(server_addr).unwrap();
    let tls_session = tls_config.connect(tcp_stream).unwrap();

    let mut client: H2Client<TlsSessionOps> = H2Client::new(tls_session).unwrap();
    client.connect().unwrap();

    let response = client.get("/").unwrap();
    assert_eq!(response.status(), 200);

    // Second request should fail due to GOAWAY
    let result = client.get("/");
    assert!(result.is_err());

    server_handle.join().unwrap();
}

#[test]
fn test_h2_tls_version_verification() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let server_addr = listener.local_addr().unwrap();

    let server_handle = thread::spawn(move || {
        let tls_config = TlsConfig::server()
            .version_range(TlsVersion::Tls12, TlsVersion::Tls13)
            .alpn(&["h2"])
            .unwrap()
            .build()
            .unwrap();

        let (tcp_stream, _) = listener.accept().unwrap();
        let tls_session = tls_config.accept(tcp_stream).unwrap();

        let vars = tls_session.vars();
        // Should be TLSv1.3 or TLSv1.2
        assert!(vars.version.contains("TLS"));

        let mut server: H2Server<TlsSessionOps> = H2Server::new(tls_session).unwrap();
        server.accept().unwrap();

        let request = server.recv_request().unwrap();
        server.send_response(request.stream_id, 200, &[], Bytes::from("OK")).unwrap();
    });

    thread::sleep(Duration::from_millis(100));

    let tls_config = TlsConfig::client()
        .version(TlsVersion::Tls13)
        .alpn(&["h2"])
        .unwrap()
        .verify_peer(false)
        .build()
        .unwrap();

    let tcp_stream = TcpStream::connect(server_addr).unwrap();
    let tls_session = tls_config.connect(tcp_stream).unwrap();

    let vars = tls_session.vars();
    assert_eq!(vars.version, "TLSv1.3");
    assert_eq!(vars.alpn.as_deref(), Some("h2"));

    let mut client: H2Client<TlsSessionOps> = H2Client::new(tls_session).unwrap();
    client.connect().unwrap();

    let response = client.get("/").unwrap();
    assert_eq!(response.status(), 200);

    server_handle.join().unwrap();
}
