//! HTTP server implementation
//!
//! This module provides HTTP server functionality for testing.

use super::{
    chunked, Error, Headers, HttpRequest, HttpResponse, HttpSession, RequestParser, Result,
    SessionOps, Status, Version, CRLF,
};
use std::io::{Read, Write};

/// HTTP server
///
/// Provides methods for receiving requests and sending responses.
pub struct HttpServer<S: SessionOps> {
    session: HttpSession<S>,
    parser: RequestParser,
    buffer: Vec<u8>,
}

impl<S: SessionOps> HttpServer<S> {
    /// Create a new HTTP server with a session
    pub fn new(session: S) -> Self {
        HttpServer {
            session: HttpSession::new(session),
            parser: RequestParser::new(),
            buffer: Vec::with_capacity(8192),
        }
    }

    /// Set the timeout for operations
    pub fn set_timeout(&mut self, timeout: std::time::Duration) {
        self.session.set_timeout(Some(timeout));
    }

    /// Receive an HTTP request (rxreq in VTC)
    pub fn receive_request(&mut self) -> Result<HttpRequest> {
        self.parser = RequestParser::new();
        self.buffer.clear();

        loop {
            let mut temp = vec![0u8; 4096];
            let n = self.session.read(&mut temp)?;

            if n == 0 {
                return Err(Error::ConnectionClosed);
            }

            self.buffer.extend_from_slice(&temp[..n]);

            if let Some(request) = self.parser.parse(&temp[..n])? {
                // We got a complete request
                return Ok(request);
            }
        }
    }

    /// Receive request headers only (rxreqhdrs in VTC)
    pub fn receive_request_headers(&mut self) -> Result<HttpRequest> {
        // For now, similar to receive_request but could be optimized
        self.receive_request()
    }

    /// Receive request body after headers
    pub fn receive_body(&mut self, headers: &Headers) -> Result<Vec<u8>> {
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

        // No body
        Ok(Vec::new())
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

    /// Send an HTTP response (txresp in VTC)
    pub fn send_response(&mut self, response: &HttpResponse) -> Result<()> {
        let wire = response.to_wire();
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

    /// Send a simple 200 OK response
    pub fn send_ok(&mut self, body: &[u8]) -> Result<()> {
        let response = HttpResponse::builder()
            .status(Status::OK)
            .header("Content-Length", body.len().to_string())
            .header("Content-Type", "text/plain")
            .body(body.to_vec())
            .build();

        self.send_response(&response)
    }

    /// Send a simple error response
    pub fn send_error(&mut self, status: Status, message: &str) -> Result<()> {
        let response = HttpResponse::builder()
            .status(status)
            .header("Content-Length", message.len().to_string())
            .header("Content-Type", "text/plain")
            .body(message.as_bytes().to_vec())
            .build();

        self.send_response(&response)
    }

    /// Send response headers only
    pub fn send_response_headers(&mut self, response: &HttpResponse) -> Result<()> {
        let mut wire = Vec::new();

        // Status line
        wire.extend_from_slice(response.version().as_str().as_bytes());
        wire.push(b' ');
        wire.extend_from_slice(response.status().code().to_string().as_bytes());
        wire.push(b' ');
        wire.extend_from_slice(response.reason().as_bytes());
        wire.extend_from_slice(CRLF.as_bytes());

        // Headers
        for (name, value) in response.headers().iter() {
            wire.extend_from_slice(name.as_bytes());
            wire.extend_from_slice(b": ");
            wire.extend_from_slice(value.as_bytes());
            wire.extend_from_slice(CRLF.as_bytes());
        }

        // Empty line
        wire.extend_from_slice(CRLF.as_bytes());

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

    /// Send response body
    pub fn send_body(&mut self, body: &[u8]) -> Result<()> {
        let mut written = 0;
        while written < body.len() {
            let n = self.session.write(&body[written..])?;
            if n == 0 {
                return Err(Error::ConnectionClosed);
            }
            written += n;
        }

        Ok(())
    }

    /// Send chunked response
    pub fn send_chunked_response(
        &mut self,
        status: Status,
        headers: &Headers,
        chunks: &[&[u8]],
    ) -> Result<()> {
        // Build response with Transfer-Encoding: chunked
        let mut response_headers = headers.clone();
        response_headers.remove("Content-Length");
        response_headers.insert("Transfer-Encoding", "chunked");

        let response = HttpResponse::builder()
            .status(status)
            .build();

        let mut resp = response;
        *resp.headers_mut() = response_headers;

        // Send headers
        self.send_response_headers(&resp)?;

        // Send chunks
        let mut buf = Vec::new();
        let mut encoder = chunked::ChunkedEncoder::new(&mut buf);

        for chunk in chunks {
            encoder.write_chunk(chunk)?;
        }
        encoder.finish()?;

        self.send_body(&buf)
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
    use crate::http::Method;
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    #[test]
    fn test_receive_request() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            let mut stream = TcpStream::connect(addr).unwrap();
            stream
                .write_all(b"GET /test HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .unwrap();

            // Read response
            let mut buf = vec![0u8; 1024];
            stream.read(&mut buf).unwrap();
        });

        let (stream, _) = listener.accept().unwrap();
        let session = FdSessionOps::new(stream);
        let mut server = HttpServer::new(session);

        let request = server.receive_request().unwrap();
        assert_eq!(request.method(), Method::Get);
        assert_eq!(request.uri(), "/test");
        assert_eq!(request.headers().get("Host"), Some("localhost"));

        // Send a response
        server.send_ok(b"OK").unwrap();

        handle.join().unwrap();
    }

    #[test]
    fn test_send_response() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            let mut stream = TcpStream::connect(addr).unwrap();
            stream
                .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .unwrap();

            // Read response
            let mut buf = vec![0u8; 1024];
            let n = stream.read(&mut buf).unwrap();
            let response = String::from_utf8_lossy(&buf[..n]);

            assert!(response.contains("HTTP/1.1 200 OK"));
            assert!(response.contains("Content-Length: 5"));
            assert!(response.contains("Hello"));
        });

        let (stream, _) = listener.accept().unwrap();
        let session = FdSessionOps::new(stream);
        let mut server = HttpServer::new(session);

        let _request = server.receive_request().unwrap();

        let response = HttpResponse::builder()
            .status(Status::OK)
            .header("Content-Length", "5")
            .body(b"Hello".to_vec())
            .build();

        server.send_response(&response).unwrap();

        handle.join().unwrap();
    }

    #[test]
    fn test_send_ok_helper() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            let mut stream = TcpStream::connect(addr).unwrap();
            stream
                .write_all(b"GET / HTTP/1.1\r\n\r\n")
                .unwrap();

            let mut buf = vec![0u8; 1024];
            let n = stream.read(&mut buf).unwrap();
            let response = String::from_utf8_lossy(&buf[..n]);

            assert!(response.contains("200 OK"));
            assert!(response.contains("Test Body"));
        });

        let (stream, _) = listener.accept().unwrap();
        let session = FdSessionOps::new(stream);
        let mut server = HttpServer::new(session);

        let _request = server.receive_request().unwrap();
        server.send_ok(b"Test Body").unwrap();

        handle.join().unwrap();
    }
}
