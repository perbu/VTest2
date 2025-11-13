//! HTTP message types
//!
//! This module defines the core types for HTTP requests and responses.

use super::{Error, Result, Headers, CRLF};
use std::fmt;

/// HTTP methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl Method {
    /// Parse method from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "GET" => Ok(Method::Get),
            "HEAD" => Ok(Method::Head),
            "POST" => Ok(Method::Post),
            "PUT" => Ok(Method::Put),
            "DELETE" => Ok(Method::Delete),
            "CONNECT" => Ok(Method::Connect),
            "OPTIONS" => Ok(Method::Options),
            "TRACE" => Ok(Method::Trace),
            "PATCH" => Ok(Method::Patch),
            _ => Err(Error::InvalidMethod(s.to_string())),
        }
    }

    /// Convert method to string
    pub fn as_str(&self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Head => "HEAD",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Connect => "CONNECT",
            Method::Options => "OPTIONS",
            Method::Trace => "TRACE",
            Method::Patch => "PATCH",
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// HTTP version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Version {
    Http10,
    Http11,
}

impl Version {
    /// Parse version from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "HTTP/1.0" => Ok(Version::Http10),
            "HTTP/1.1" => Ok(Version::Http11),
            _ => Err(Error::InvalidVersion(s.to_string())),
        }
    }

    /// Convert version to string
    pub fn as_str(&self) -> &'static str {
        match self {
            Version::Http10 => "HTTP/1.0",
            Version::Http11 => "HTTP/1.1",
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Default for Version {
    fn default() -> Self {
        Version::Http11
    }
}

/// HTTP status code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Status {
    code: u16,
}

impl Status {
    /// Create a new status code
    pub fn new(code: u16) -> Result<Self> {
        if (100..600).contains(&code) {
            Ok(Status { code })
        } else {
            Err(Error::InvalidStatus(format!("Invalid status code: {}", code)))
        }
    }

    /// Get the status code
    pub fn code(&self) -> u16 {
        self.code
    }

    /// Get the canonical reason phrase for this status code
    pub fn reason_phrase(&self) -> &'static str {
        match self.code {
            100 => "Continue",
            101 => "Switching Protocols",
            200 => "OK",
            201 => "Created",
            202 => "Accepted",
            203 => "Non-Authoritative Information",
            204 => "No Content",
            205 => "Reset Content",
            206 => "Partial Content",
            300 => "Multiple Choices",
            301 => "Moved Permanently",
            302 => "Found",
            303 => "See Other",
            304 => "Not Modified",
            305 => "Use Proxy",
            307 => "Temporary Redirect",
            308 => "Permanent Redirect",
            400 => "Bad Request",
            401 => "Unauthorized",
            402 => "Payment Required",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            406 => "Not Acceptable",
            407 => "Proxy Authentication Required",
            408 => "Request Timeout",
            409 => "Conflict",
            410 => "Gone",
            411 => "Length Required",
            412 => "Precondition Failed",
            413 => "Payload Too Large",
            414 => "URI Too Long",
            415 => "Unsupported Media Type",
            416 => "Range Not Satisfiable",
            417 => "Expectation Failed",
            426 => "Upgrade Required",
            500 => "Internal Server Error",
            501 => "Not Implemented",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            504 => "Gateway Timeout",
            505 => "HTTP Version Not Supported",
            _ => "Unknown",
        }
    }

    /// Check if this is an informational status (1xx)
    pub fn is_informational(&self) -> bool {
        (100..200).contains(&self.code)
    }

    /// Check if this is a success status (2xx)
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.code)
    }

    /// Check if this is a redirection status (3xx)
    pub fn is_redirection(&self) -> bool {
        (300..400).contains(&self.code)
    }

    /// Check if this is a client error status (4xx)
    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.code)
    }

    /// Check if this is a server error status (5xx)
    pub fn is_server_error(&self) -> bool {
        (500..600).contains(&self.code)
    }

    // Common status codes as constants
    pub const OK: Status = Status { code: 200 };
    pub const NOT_FOUND: Status = Status { code: 404 };
    pub const INTERNAL_SERVER_ERROR: Status = Status { code: 500 };
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.code, self.reason_phrase())
    }
}

/// HTTP request
#[derive(Debug, Clone)]
pub struct HttpRequest {
    method: Method,
    uri: String,
    version: Version,
    headers: Headers,
    body: Vec<u8>,
}

impl HttpRequest {
    /// Create a new HTTP request
    pub fn new(method: Method, uri: impl Into<String>) -> Self {
        HttpRequest {
            method,
            uri: uri.into(),
            version: Version::default(),
            headers: Headers::new(),
            body: Vec::new(),
        }
    }

    /// Create a builder for constructing requests
    pub fn builder() -> HttpRequestBuilder {
        HttpRequestBuilder::default()
    }

    /// Get the request method
    pub fn method(&self) -> Method {
        self.method
    }

    /// Get the request URI
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Get the HTTP version
    pub fn version(&self) -> Version {
        self.version
    }

    /// Get the headers
    pub fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Get mutable headers
    pub fn headers_mut(&mut self) -> &mut Headers {
        &mut self.headers
    }

    /// Get the body
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Set the body
    pub fn set_body(&mut self, body: Vec<u8>) {
        self.body = body;
    }

    /// Convert the request to wire format
    pub fn to_wire(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // Request line
        buf.extend_from_slice(self.method.as_str().as_bytes());
        buf.push(b' ');
        buf.extend_from_slice(self.uri.as_bytes());
        buf.push(b' ');
        buf.extend_from_slice(self.version.as_str().as_bytes());
        buf.extend_from_slice(CRLF.as_bytes());

        // Headers
        for (name, value) in self.headers.iter() {
            buf.extend_from_slice(name.as_bytes());
            buf.extend_from_slice(b": ");
            buf.extend_from_slice(value.as_bytes());
            buf.extend_from_slice(CRLF.as_bytes());
        }

        // Empty line
        buf.extend_from_slice(CRLF.as_bytes());

        // Body
        buf.extend_from_slice(&self.body);

        buf
    }
}

/// Builder for HTTP requests
#[derive(Debug, Default)]
pub struct HttpRequestBuilder {
    method: Option<Method>,
    uri: Option<String>,
    version: Option<Version>,
    headers: Headers,
    body: Vec<u8>,
}

impl HttpRequestBuilder {
    /// Set the HTTP method
    pub fn method(mut self, method: Method) -> Self {
        self.method = Some(method);
        self
    }

    /// Set the URI
    pub fn uri(mut self, uri: impl Into<String>) -> Self {
        self.uri = Some(uri.into());
        self
    }

    /// Set the HTTP version
    pub fn version(mut self, version: Version) -> Self {
        self.version = Some(version);
        self
    }

    /// Add a header
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Set the body
    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    /// Build the request
    pub fn build(self) -> HttpRequest {
        HttpRequest {
            method: self.method.unwrap_or(Method::Get),
            uri: self.uri.unwrap_or_else(|| "/".to_string()),
            version: self.version.unwrap_or_default(),
            headers: self.headers,
            body: self.body,
        }
    }
}

/// HTTP response
#[derive(Debug, Clone)]
pub struct HttpResponse {
    version: Version,
    status: Status,
    reason: String,
    headers: Headers,
    body: Vec<u8>,
}

impl HttpResponse {
    /// Create a new HTTP response
    pub fn new(status: Status) -> Self {
        let reason = status.reason_phrase().to_string();
        HttpResponse {
            version: Version::default(),
            status,
            reason,
            headers: Headers::new(),
            body: Vec::new(),
        }
    }

    /// Create a builder for constructing responses
    pub fn builder() -> HttpResponseBuilder {
        HttpResponseBuilder::default()
    }

    /// Get the HTTP version
    pub fn version(&self) -> Version {
        self.version
    }

    /// Get the status code
    pub fn status(&self) -> Status {
        self.status
    }

    /// Get the reason phrase
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// Get the headers
    pub fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Get mutable headers
    pub fn headers_mut(&mut self) -> &mut Headers {
        &mut self.headers
    }

    /// Get the body
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Set the body
    pub fn set_body(&mut self, body: Vec<u8>) {
        self.body = body;
    }

    /// Convert the response to wire format
    pub fn to_wire(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // Status line
        buf.extend_from_slice(self.version.as_str().as_bytes());
        buf.push(b' ');
        buf.extend_from_slice(self.status.code().to_string().as_bytes());
        buf.push(b' ');
        buf.extend_from_slice(self.reason.as_bytes());
        buf.extend_from_slice(CRLF.as_bytes());

        // Headers
        for (name, value) in self.headers.iter() {
            buf.extend_from_slice(name.as_bytes());
            buf.extend_from_slice(b": ");
            buf.extend_from_slice(value.as_bytes());
            buf.extend_from_slice(CRLF.as_bytes());
        }

        // Empty line
        buf.extend_from_slice(CRLF.as_bytes());

        // Body
        buf.extend_from_slice(&self.body);

        buf
    }
}

/// Builder for HTTP responses
#[derive(Debug)]
pub struct HttpResponseBuilder {
    version: Option<Version>,
    status: Option<Status>,
    reason: Option<String>,
    headers: Headers,
    body: Vec<u8>,
}

impl Default for HttpResponseBuilder {
    fn default() -> Self {
        HttpResponseBuilder {
            version: None,
            status: None,
            reason: None,
            headers: Headers::new(),
            body: Vec::new(),
        }
    }
}

impl HttpResponseBuilder {
    /// Set the HTTP version
    pub fn version(mut self, version: Version) -> Self {
        self.version = Some(version);
        self
    }

    /// Set the status code
    pub fn status(mut self, status: Status) -> Self {
        self.status = Some(status);
        self
    }

    /// Set the reason phrase
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Add a header
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Set the body
    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    /// Build the response
    pub fn build(self) -> HttpResponse {
        let status = self.status.unwrap_or(Status::OK);
        let reason = self.reason.unwrap_or_else(|| status.reason_phrase().to_string());
        HttpResponse {
            version: self.version.unwrap_or_default(),
            status,
            reason,
            headers: self.headers,
            body: self.body,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_from_str() {
        assert_eq!(Method::from_str("GET").unwrap(), Method::Get);
        assert_eq!(Method::from_str("POST").unwrap(), Method::Post);
        assert!(Method::from_str("INVALID").is_err());
    }

    #[test]
    fn test_version_from_str() {
        assert_eq!(Version::from_str("HTTP/1.0").unwrap(), Version::Http10);
        assert_eq!(Version::from_str("HTTP/1.1").unwrap(), Version::Http11);
        assert!(Version::from_str("HTTP/2.0").is_err());
    }

    #[test]
    fn test_status() {
        let status = Status::new(200).unwrap();
        assert_eq!(status.code(), 200);
        assert_eq!(status.reason_phrase(), "OK");
        assert!(status.is_success());
        assert!(!status.is_client_error());
    }

    #[test]
    fn test_request_builder() {
        let req = HttpRequest::builder()
            .method(Method::Post)
            .uri("/test")
            .header("Content-Type", "text/plain")
            .body(b"Hello".to_vec())
            .build();

        assert_eq!(req.method(), Method::Post);
        assert_eq!(req.uri(), "/test");
        assert_eq!(req.body(), b"Hello");
        assert_eq!(req.headers().get("Content-Type"), Some("text/plain"));
    }

    #[test]
    fn test_response_builder() {
        let resp = HttpResponse::builder()
            .status(Status::new(404).unwrap())
            .header("Content-Type", "text/html")
            .body(b"Not Found".to_vec())
            .build();

        assert_eq!(resp.status().code(), 404);
        assert_eq!(resp.body(), b"Not Found");
    }

    #[test]
    fn test_request_to_wire() {
        let req = HttpRequest::builder()
            .method(Method::Get)
            .uri("/")
            .header("Host", "example.com")
            .build();

        let wire = String::from_utf8(req.to_wire()).unwrap();
        assert!(wire.starts_with("GET / HTTP/1.1\r\n"));
        assert!(wire.contains("Host: example.com\r\n"));
        assert!(wire.contains("\r\n\r\n"));
    }

    #[test]
    fn test_response_to_wire() {
        let resp = HttpResponse::builder()
            .status(Status::new(200).unwrap())
            .header("Content-Length", "0")
            .build();

        let wire = String::from_utf8(resp.to_wire()).unwrap();
        assert!(wire.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(wire.contains("Content-Length: 0\r\n"));
        assert!(wire.contains("\r\n\r\n"));
    }
}
