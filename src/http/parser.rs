//! HTTP message parsing
//!
//! This module provides parsers for HTTP requests and responses.

use super::{Error, Result, Headers, HttpRequest, HttpResponse, Method, Status, Version, CRLF};

/// Find the next CRLF in a buffer
fn find_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(2).position(|w| w == b"\r\n")
}

/// Parse HTTP request line
///
/// Format: METHOD URI VERSION\r\n
/// Example: GET /index.html HTTP/1.1\r\n
pub fn parse_request_line(line: &str) -> Result<(Method, String, Version)> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.len() != 3 {
        return Err(Error::Parse(format!(
            "Invalid request line: expected 3 parts, got {}",
            parts.len()
        )));
    }

    let method = Method::from_str(parts[0])?;
    let uri = parts[1].to_string();
    let version = Version::from_str(parts[2])?;

    Ok((method, uri, version))
}

/// Parse HTTP response status line
///
/// Format: VERSION STATUS REASON\r\n
/// Example: HTTP/1.1 200 OK\r\n
pub fn parse_status_line(line: &str) -> Result<(Version, Status, String)> {
    let parts: Vec<&str> = line.splitn(3, ' ').collect();

    if parts.len() < 2 {
        return Err(Error::Parse(format!(
            "Invalid status line: expected at least 2 parts, got {}",
            parts.len()
        )));
    }

    let version = Version::from_str(parts[0])?;
    let status_code = parts[1]
        .parse::<u16>()
        .map_err(|_| Error::Parse(format!("Invalid status code: {}", parts[1])))?;
    let status = Status::new(status_code)?;
    let reason = if parts.len() == 3 {
        parts[2].to_string()
    } else {
        status.reason_phrase().to_string()
    };

    Ok((version, status, reason))
}

/// HTTP request parser
pub struct RequestParser {
    state: ParserState,
    buffer: Vec<u8>,
    method: Option<Method>,
    uri: Option<String>,
    version: Option<Version>,
    headers: Headers,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ParserState {
    RequestLine,
    Headers,
    Body,
    Complete,
}

impl RequestParser {
    /// Create a new request parser
    pub fn new() -> Self {
        RequestParser {
            state: ParserState::RequestLine,
            buffer: Vec::new(),
            method: None,
            uri: None,
            version: None,
            headers: Headers::new(),
        }
    }

    /// Feed data to the parser
    ///
    /// Returns Ok(Some(request)) when a complete request is parsed,
    /// Ok(None) if more data is needed, or Err on parse error.
    pub fn parse(&mut self, data: &[u8]) -> Result<Option<HttpRequest>> {
        self.buffer.extend_from_slice(data);

        match self.state {
            ParserState::RequestLine => self.parse_request_line(),
            ParserState::Headers => self.parse_headers(),
            ParserState::Body => self.parse_body(),
            ParserState::Complete => Ok(None),
        }
    }

    fn parse_request_line(&mut self) -> Result<Option<HttpRequest>> {
        if let Some(crlf_pos) = find_crlf(&self.buffer) {
            let line = String::from_utf8_lossy(&self.buffer[..crlf_pos]).to_string();
            self.buffer.drain(..crlf_pos + 2);

            let (method, uri, version) = parse_request_line(&line)?;

            // Store components
            self.method = Some(method);
            self.uri = Some(uri);
            self.version = Some(version);

            self.state = ParserState::Headers;
            self.parse_headers()
        } else {
            Ok(None)
        }
    }

    fn parse_headers(&mut self) -> Result<Option<HttpRequest>> {
        loop {
            if let Some(crlf_pos) = find_crlf(&self.buffer) {
                if crlf_pos == 0 {
                    // Empty line marks end of headers
                    self.buffer.drain(..2);
                    self.state = ParserState::Body;
                    return self.parse_body();
                }

                let line = String::from_utf8_lossy(&self.buffer[..crlf_pos]).to_string();
                self.buffer.drain(..crlf_pos + 2);

                let (name, value) = Headers::parse_header_line(&line)?;
                self.headers.insert(name, value);
            } else {
                return Ok(None);
            }
        }
    }

    fn parse_body(&mut self) -> Result<Option<HttpRequest>> {
        // For requests, we typically wait for explicit body handling
        // For now, we'll just check Content-Length
        if let Some(cl_str) = self.headers.get("Content-Length") {
            let content_length = cl_str
                .parse::<usize>()
                .map_err(|_| Error::Parse(format!("Invalid Content-Length: {}", cl_str)))?;

            if self.buffer.len() >= content_length {
                let body = self.buffer.drain(..content_length).collect();
                self.state = ParserState::Complete;

                // Construct request from stored components
                let req = HttpRequest::builder()
                    .method(self.method.unwrap())
                    .uri(self.uri.as_ref().unwrap())
                    .version(self.version.unwrap())
                    .body(body)
                    .build();

                let mut req_with_headers = req;
                *req_with_headers.headers_mut() = self.headers.clone();

                return Ok(Some(req_with_headers));
            } else {
                return Ok(None);
            }
        }

        // No body expected
        self.state = ParserState::Complete;
        let req = HttpRequest::builder()
            .method(self.method.unwrap())
            .uri(self.uri.as_ref().unwrap())
            .version(self.version.unwrap())
            .build();

        let mut req_with_headers = req;
        *req_with_headers.headers_mut() = self.headers.clone();
        Ok(Some(req_with_headers))
    }
}

impl Default for RequestParser {
    fn default() -> Self {
        Self::new()
    }
}

/// HTTP response parser
pub struct ResponseParser {
    state: ParserState,
    buffer: Vec<u8>,
    version: Option<Version>,
    status: Option<Status>,
    reason: Option<String>,
    headers: Headers,
}

impl ResponseParser {
    /// Create a new response parser
    pub fn new() -> Self {
        ResponseParser {
            state: ParserState::RequestLine, // Reuse for status line
            buffer: Vec::new(),
            version: None,
            status: None,
            reason: None,
            headers: Headers::new(),
        }
    }

    /// Feed data to the parser
    ///
    /// Returns Ok(Some(response)) when a complete response is parsed,
    /// Ok(None) if more data is needed, or Err on parse error.
    pub fn parse(&mut self, data: &[u8]) -> Result<Option<HttpResponse>> {
        self.buffer.extend_from_slice(data);

        match self.state {
            ParserState::RequestLine => self.parse_status_line(),
            ParserState::Headers => self.parse_headers(),
            ParserState::Body => self.parse_body(),
            ParserState::Complete => Ok(None),
        }
    }

    fn parse_status_line(&mut self) -> Result<Option<HttpResponse>> {
        if let Some(crlf_pos) = find_crlf(&self.buffer) {
            let line = String::from_utf8_lossy(&self.buffer[..crlf_pos]).to_string();
            self.buffer.drain(..crlf_pos + 2);

            let (version, status, reason) = parse_status_line(&line)?;
            self.version = Some(version);
            self.status = Some(status);
            self.reason = Some(reason);

            self.state = ParserState::Headers;
            self.parse_headers()
        } else {
            Ok(None)
        }
    }

    fn parse_headers(&mut self) -> Result<Option<HttpResponse>> {
        loop {
            if let Some(crlf_pos) = find_crlf(&self.buffer) {
                if crlf_pos == 0 {
                    // Empty line marks end of headers
                    self.buffer.drain(..2);
                    self.state = ParserState::Body;
                    return self.parse_body();
                }

                let line = String::from_utf8_lossy(&self.buffer[..crlf_pos]).to_string();
                self.buffer.drain(..crlf_pos + 2);

                let (name, value) = Headers::parse_header_line(&line)?;
                self.headers.insert(name, value);
            } else {
                return Ok(None);
            }
        }
    }

    fn parse_body(&mut self) -> Result<Option<HttpResponse>> {
        // Check for Content-Length
        if let Some(cl_str) = self.headers.get("Content-Length") {
            let content_length = cl_str
                .parse::<usize>()
                .map_err(|_| Error::Parse(format!("Invalid Content-Length: {}", cl_str)))?;

            if self.buffer.len() >= content_length {
                let body = self.buffer.drain(..content_length).collect();
                self.state = ParserState::Complete;

                let resp = HttpResponse::builder()
                    .version(self.version.unwrap())
                    .status(self.status.unwrap())
                    .reason(self.reason.take().unwrap())
                    .body(body)
                    .build();

                let mut resp_with_headers = resp;
                *resp_with_headers.headers_mut() = self.headers.clone();

                return Ok(Some(resp_with_headers));
            } else {
                return Ok(None);
            }
        }

        // No body expected
        self.state = ParserState::Complete;
        let resp = HttpResponse::builder()
            .version(self.version.unwrap())
            .status(self.status.unwrap())
            .reason(self.reason.take().unwrap())
            .build();

        let mut resp_with_headers = resp;
        *resp_with_headers.headers_mut() = self.headers.clone();
        Ok(Some(resp_with_headers))
    }

    /// Reset the parser for reuse
    pub fn reset(&mut self) {
        self.state = ParserState::RequestLine;
        self.buffer.clear();
        self.version = None;
        self.status = None;
        self.reason = None;
        self.headers.clear();
    }
}

impl Default for ResponseParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_request_line() {
        let (method, uri, version) = parse_request_line("GET /index.html HTTP/1.1").unwrap();
        assert_eq!(method, Method::Get);
        assert_eq!(uri, "/index.html");
        assert_eq!(version, Version::Http11);
    }

    #[test]
    fn test_parse_status_line() {
        let (version, status, reason) = parse_status_line("HTTP/1.1 200 OK").unwrap();
        assert_eq!(version, Version::Http11);
        assert_eq!(status.code(), 200);
        assert_eq!(reason, "OK");

        // Test without reason phrase
        let (version, status, reason) = parse_status_line("HTTP/1.0 404").unwrap();
        assert_eq!(version, Version::Http10);
        assert_eq!(status.code(), 404);
        assert_eq!(reason, "Not Found");
    }

    #[test]
    fn test_response_parser_simple() {
        let mut parser = ResponseParser::new();

        let data = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nHello";
        let response = parser.parse(data).unwrap();

        assert!(response.is_some());
        let resp = response.unwrap();
        assert_eq!(resp.status().code(), 200);
        assert_eq!(resp.body(), b"Hello");
        assert_eq!(resp.headers().get("Content-Length"), Some("5"));
    }

    #[test]
    fn test_response_parser_incremental() {
        let mut parser = ResponseParser::new();

        // Feed data incrementally
        assert!(parser.parse(b"HTTP/1.1 ").unwrap().is_none());
        assert!(parser.parse(b"200 OK\r\n").unwrap().is_none());
        assert!(parser.parse(b"Content-Type: text/plain\r\n").unwrap().is_none());
        assert!(parser.parse(b"Content-Length: 4\r\n\r\n").unwrap().is_none());
        let response = parser.parse(b"Test").unwrap();

        assert!(response.is_some());
        let resp = response.unwrap();
        assert_eq!(resp.body(), b"Test");
        assert_eq!(resp.headers().get("Content-Type"), Some("text/plain"));
    }

    #[test]
    fn test_find_crlf() {
        assert_eq!(find_crlf(b"Hello\r\nWorld"), Some(5));
        assert_eq!(find_crlf(b"NoEOL"), None);
        assert_eq!(find_crlf(b"\r\n"), Some(0));
        assert_eq!(find_crlf(b"First\r\nSecond\r\n"), Some(5));
    }
}
