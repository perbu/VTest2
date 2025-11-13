//! Integration tests for the HTTP layer
//!
//! These tests verify end-to-end functionality of HTTP client and server.

use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;
use vtest2::http::{HttpClient, HttpRequest, HttpResponse, HttpServer, Method, Status};
use vtest2::http::session::FdSessionOps;

#[test]
fn test_http_request_response_cycle() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    // Server thread
    let server_handle = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let session = FdSessionOps::new(stream);
        let mut server = HttpServer::new(session);

        // Receive request
        let request = server.receive_request().unwrap();
        assert_eq!(request.method(), Method::Get);
        assert_eq!(request.uri(), "/test");
        assert_eq!(request.headers().get("Host"), Some("localhost"));

        // Send response
        let response = HttpResponse::builder()
            .status(Status::OK)
            .header("Content-Type", "text/plain")
            .header("Content-Length", "11")
            .body(b"Hello World".to_vec())
            .build();

        server.send_response(&response).unwrap();
    });

    // Client thread
    let client_handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50)); // Give server time to start

        let stream = TcpStream::connect(addr).unwrap();
        let session = FdSessionOps::new(stream);
        let mut client = HttpClient::new(session);

        // Send request
        let request = HttpRequest::builder()
            .method(Method::Get)
            .uri("/test")
            .header("Host", "localhost")
            .build();

        client.send_request(&request).unwrap();

        // Receive response
        let response = client.receive_response().unwrap();
        assert_eq!(response.status().code(), 200);
        assert_eq!(response.headers().get("Content-Type"), Some("text/plain"));
        assert_eq!(response.body(), b"Hello World");
    });

    server_handle.join().unwrap();
    client_handle.join().unwrap();
}

#[test]
fn test_http_post_with_body() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let server_handle = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let session = FdSessionOps::new(stream);
        let mut server = HttpServer::new(session);

        let request = server.receive_request().unwrap();
        assert_eq!(request.method(), Method::Post);
        assert_eq!(request.uri(), "/data");
        assert_eq!(request.body(), b"test data");

        server.send_ok(b"Received").unwrap();
    });

    let client_handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));

        let stream = TcpStream::connect(addr).unwrap();
        let session = FdSessionOps::new(stream);
        let mut client = HttpClient::new(session);

        let response = client.post("/data", b"test data".to_vec()).unwrap();
        assert_eq!(response.status().code(), 200);
        assert_eq!(response.body(), b"Received");
    });

    server_handle.join().unwrap();
    client_handle.join().unwrap();
}

#[test]
fn test_http_large_body() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let test_body = "Hello World".repeat(100);
    let expected_body = test_body.clone();

    let server_handle = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let session = FdSessionOps::new(stream);
        let mut server = HttpServer::new(session);

        let _request = server.receive_request().unwrap();

        // Send response with larger body
        let response = HttpResponse::builder()
            .status(Status::OK)
            .header("Content-Length", test_body.len().to_string())
            .body(test_body.as_bytes().to_vec())
            .build();

        server.send_response(&response).unwrap();
    });

    let client_handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));

        let stream = TcpStream::connect(addr).unwrap();
        let session = FdSessionOps::new(stream);
        let mut client = HttpClient::new(session);

        let response = client.get("/").unwrap();
        assert_eq!(response.status().code(), 200);
        assert_eq!(response.body(), expected_body.as_bytes());
    });

    server_handle.join().unwrap();
    client_handle.join().unwrap();
}

#[test]
fn test_multiple_requests_on_same_connection() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let server_handle = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let session = FdSessionOps::new(stream);
        let mut server = HttpServer::new(session);

        // Handle multiple requests
        for i in 1..=3 {
            let request = server.receive_request().unwrap();
            assert_eq!(request.method(), Method::Get);

            let body = format!("Response {}", i);
            server.send_ok(body.as_bytes()).unwrap();
        }
    });

    let client_handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));

        let stream = TcpStream::connect(addr).unwrap();
        let session = FdSessionOps::new(stream);
        let mut client = HttpClient::new(session);

        // Send multiple requests
        for i in 1..=3 {
            let response = client.get("/").unwrap();
            assert_eq!(response.status().code(), 200);
            assert_eq!(response.body(), format!("Response {}", i).as_bytes());
        }
    });

    server_handle.join().unwrap();
    client_handle.join().unwrap();
}

#[test]
fn test_http_headers_case_insensitive() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let server_handle = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let session = FdSessionOps::new(stream);
        let mut server = HttpServer::new(session);

        let request = server.receive_request().unwrap();

        // Test case-insensitive header access
        assert_eq!(request.headers().get("content-type"), Some("application/json"));
        assert_eq!(request.headers().get("CONTENT-TYPE"), Some("application/json"));
        assert_eq!(request.headers().get("Content-Type"), Some("application/json"));

        server.send_ok(b"OK").unwrap();
    });

    let client_handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));

        let stream = TcpStream::connect(addr).unwrap();
        let session = FdSessionOps::new(stream);
        let mut client = HttpClient::new(session);

        let request = HttpRequest::builder()
            .method(Method::Post)
            .uri("/")
            .header("Content-Type", "application/json")
            .build();

        client.send_request(&request).unwrap();
        let response = client.receive_response().unwrap();
        assert_eq!(response.status().code(), 200);
    });

    server_handle.join().unwrap();
    client_handle.join().unwrap();
}

#[test]
fn test_http_404_error() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let server_handle = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let session = FdSessionOps::new(stream);
        let mut server = HttpServer::new(session);

        let request = server.receive_request().unwrap();
        assert_eq!(request.uri(), "/notfound");

        // Send 404
        server.send_error(Status::NOT_FOUND, "Not Found").unwrap();
    });

    let client_handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));

        let stream = TcpStream::connect(addr).unwrap();
        let session = FdSessionOps::new(stream);
        let mut client = HttpClient::new(session);

        let response = client.get("/notfound").unwrap();
        assert_eq!(response.status().code(), 404);
        assert_eq!(response.body(), b"Not Found");
    });

    server_handle.join().unwrap();
    client_handle.join().unwrap();
}
