//! HTTP client implementation
//!
//! This module provides HTTP client functionality for testing.

use super::{
    chunked, Error, Headers, HttpRequest, HttpResponse, HttpSession, Method, ResponseParser,
    Result, SessionOps, Status, Version, CRLF,
};
use std::io::{Read, Write};

/// HTTP client
///
/// Provides methods for sending requests and receiving responses.
pub struct HttpClient<S: SessionOps> {
    session: HttpSession<S>,
    parser: ResponseParser,
    buffer: Vec<u8>,
}

impl<S: SessionOps> HttpClient<S> {
    /// Create a new HTTP client with a session
    pub fn new(session: S) -> Self {
        HttpClient {
            session: HttpSession::new(session),
            parser: ResponseParser::new(),
            buffer: Vec::with_capacity(8192),
        }
    }

    /// Set the timeout for operations
    pub fn set_timeout(&mut self, timeout: std::time::Duration) {
        self.session.set_timeout(Some(timeout));
    }

    /// Send an HTTP request (txreq in VTC)
    pub fn send_request(&mut self, request: &HttpRequest) -> Result<()> {
        let wire = request.to_wire();
        let mut written = 0;

        while written < wire.len() {
            let n = self.session.write(&wire[written..])?;
            if n == 0 {
                return Err(Error::ConnectionClosed);
            }
            written += n;
        }

        Ok(())
    }

    /// Receive an HTTP response (rxresp in VTC)
    pub fn receive_response(&mut self) -> Result<HttpResponse> {
        self.parser.reset();
        self.buffer.clear();

        loop {
            let mut temp = vec![0u8; 4096];
            let n = self.session.read(&mut temp)?;

            if n == 0 {
                return Err(Error::ConnectionClosed);
            }

            self.buffer.extend_from_slice(&temp[..n]);

            if let Some(response) = self.parser.parse(&temp[..n])? {
                // We got a complete response
                return Ok(response);
            }
        }
    }

    /// Receive response headers only (rxresphdrs in VTC)
    pub fn receive_response_headers(&mut self) -> Result<HttpResponse> {
        // For now, this is similar to receive_response but we could optimize
        // to not read the body
        self.receive_response()
    }

    /// Receive response body after headers
    ///
    /// This is used when headers and body are received separately.
    pub fn receive_body(
        &mut self,
        headers: &Headers,
        is_head_request: bool,
    ) -> Result<Vec<u8>> {
        if is_head_request {
            // HEAD requests don't have a body
            return Ok(Vec::new());
        }

        // Check for chunked encoding
        if let Some(encoding) = headers.get("Transfer-Encoding") {
            if encoding.eq_ignore_ascii_case("chunked") {
                return self.receive_chunked_body();
            }
        }

        // Check for Content-Length
        if let Some(cl_str) = headers.get("Content-Length") {
            let content_length = cl_str
                .parse::<usize>()
                .map_err(|_| Error::Parse(format!("Invalid Content-Length: {}", cl_str)))?;

            let mut body = vec![0u8; content_length];
            let mut total_read = 0;

            while total_read < content_length {
                let n = self.session.read(&mut body[total_read..])?;
                if n == 0 {
                    return Err(Error::ConnectionClosed);
                }
                total_read += n;
            }

            return Ok(body);
        }

        // No explicit length - read until EOF
        let mut body = Vec::new();
        loop {
            let mut temp = vec![0u8; 4096];
            match self.session.read(&mut temp) {
                Ok(0) => break, // EOF
                Ok(n) => body.extend_from_slice(&temp[..n]),
                Err(Error::Timeout) => break, // Consider timeout as end
                Err(e) => return Err(e),
            }
        }

        Ok(body)
    }

    /// Receive chunked body
    fn receive_chunked_body(&mut self) -> Result<Vec<u8>> {
        let mut decoder = chunked::ChunkedDecoder::new();
        let mut output = Vec::new();
        let mut input_buffer = Vec::new();

        loop {
            // Read more data
            let mut temp = vec![0u8; 4096];
            let n = self.session.read(&mut temp)?;

            if n == 0 {
                return Err(Error::ConnectionClosed);
            }

            input_buffer.extend_from_slice(&temp[..n]);

            // Try to decode
            let mut decode_buffer = vec![0u8; 8192];
            let (consumed, decoded, complete) =
                decoder.decode(&input_buffer, &mut decode_buffer)?;

            output.extend_from_slice(&decode_buffer[..decoded]);

            // Remove consumed bytes from input
            input_buffer.drain(..consumed);

            if complete {
                break;
            }
        }

        Ok(output)
    }

    /// Send a simple GET request
    pub fn get(&mut self, uri: &str) -> Result<HttpResponse> {
        let request = HttpRequest::builder()
            .method(Method::Get)
            .uri(uri)
            .header("Host", "localhost")
            .build();

        self.send_request(&request)?;
        self.receive_response()
    }

    /// Send a simple POST request with body
    pub fn post(&mut self, uri: &str, body: Vec<u8>) -> Result<HttpResponse> {
        let request = HttpRequest::builder()
            .method(Method::Post)
            .uri(uri)
            .header("Host", "localhost")
            .header("Content-Length", body.len().to_string())
            .body(body)
            .build();

        self.send_request(&request)?;
        self.receive_response()
    }

    /// Close the connection
    pub fn close(&mut self) -> Result<()> {
        self.session.close()
    }

    /// Get a reference to the underlying session
    pub fn session(&self) -> &HttpSession<S> {
        &self.session
    }

    /// Get a mutable reference to the underlying session
    pub fn session_mut(&mut self) -> &mut HttpSession<S> {
        &mut self.session
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::session::FdSessionOps;
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    #[test]
    fn test_send_request() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0u8; 1024];
            let n = stream.read(&mut buf).unwrap();
            let request = String::from_utf8_lossy(&buf[..n]);

            assert!(request.contains("GET / HTTP/1.1"));
            assert!(request.contains("Host: localhost"));

            // Send a response
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK")
                .unwrap();
        });

        let stream = TcpStream::connect(addr).unwrap();
        let session = FdSessionOps::new(stream);
        let mut client = HttpClient::new(session);

        let request = HttpRequest::builder()
            .method(Method::Get)
            .uri("/")
            .header("Host", "localhost")
            .build();

        client.send_request(&request).unwrap();

        handle.join().unwrap();
    }

    #[test]
    fn test_receive_response() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            // Read request (but ignore it)
            let mut buf = vec![0u8; 1024];
            stream.read(&mut buf).unwrap();

            // Send response
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 5\r\n\r\nHello")
                .unwrap();
        });

        let stream = TcpStream::connect(addr).unwrap();
        let session = FdSessionOps::new(stream);
        let mut client = HttpClient::new(session);

        let request = HttpRequest::builder()
            .method(Method::Get)
            .uri("/")
            .build();

        client.send_request(&request).unwrap();
        let response = client.receive_response().unwrap();

        assert_eq!(response.status().code(), 200);
        assert_eq!(response.body(), b"Hello");
        assert_eq!(response.headers().get("Content-Type"), Some("text/plain"));

        handle.join().unwrap();
    }

    #[test]
    fn test_get_helper() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0u8; 1024];
            stream.read(&mut buf).unwrap();

            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK")
                .unwrap();
        });

        let stream = TcpStream::connect(addr).unwrap();
        let session = FdSessionOps::new(stream);
        let mut client = HttpClient::new(session);

        let response = client.get("/test").unwrap();
        assert_eq!(response.status().code(), 200);
        assert_eq!(response.body(), b"OK");

        handle.join().unwrap();
    }
}
