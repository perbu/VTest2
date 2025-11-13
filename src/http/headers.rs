//! HTTP headers handling
//!
//! This module provides a type for managing HTTP headers with case-insensitive
//! lookups and support for multiple values per header name.

use super::{Error, Result, MAX_HEADERS};
use std::fmt;

/// HTTP headers collection
///
/// Headers are stored in insertion order and support:
/// - Case-insensitive header name lookups
/// - Multiple values for the same header name
/// - Iteration over all headers
#[derive(Debug, Clone)]
pub struct Headers {
    headers: Vec<(String, String)>,
}

impl Headers {
    /// Create a new empty headers collection
    pub fn new() -> Self {
        Headers {
            headers: Vec::new(),
        }
    }

    /// Insert a header
    ///
    /// If a header with the same name (case-insensitive) already exists,
    /// this adds another value rather than replacing it.
    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<String>) {
        let name = name.into();
        let value = value.into();

        if self.headers.len() >= MAX_HEADERS {
            // Silently ignore if we've hit the max (matching C behavior)
            return;
        }

        self.headers.push((name, value));
    }

    /// Get the first value for a header (case-insensitive)
    pub fn get(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// Get all values for a header (case-insensitive)
    pub fn get_all(&self, name: &str) -> Vec<&str> {
        self.headers
            .iter()
            .filter(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
            .collect()
    }

    /// Count how many times a header appears
    pub fn count(&self, name: &str) -> usize {
        self.headers
            .iter()
            .filter(|(n, _)| n.eq_ignore_ascii_case(name))
            .count()
    }

    /// Check if a header exists
    pub fn contains(&self, name: &str) -> bool {
        self.headers
            .iter()
            .any(|(n, _)| n.eq_ignore_ascii_case(name))
    }

    /// Remove all instances of a header (case-insensitive)
    pub fn remove(&mut self, name: &str) -> usize {
        let initial_len = self.headers.len();
        self.headers.retain(|(n, _)| !n.eq_ignore_ascii_case(name));
        initial_len - self.headers.len()
    }

    /// Get the number of headers
    pub fn len(&self) -> usize {
        self.headers.len()
    }

    /// Check if there are no headers
    pub fn is_empty(&self) -> bool {
        self.headers.is_empty()
    }

    /// Clear all headers
    pub fn clear(&mut self) {
        self.headers.clear();
    }

    /// Iterate over all headers
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.headers.iter().map(|(n, v)| (n.as_str(), v.as_str()))
    }

    /// Parse a header line into name and value
    pub fn parse_header_line(line: &str) -> Result<(String, String)> {
        if let Some(colon_pos) = line.find(':') {
            let name = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();

            if name.is_empty() {
                return Err(Error::InvalidHeader("Empty header name".to_string()));
            }

            Ok((name, value))
        } else {
            Err(Error::InvalidHeader(format!("No colon in header: {}", line)))
        }
    }
}

impl Default for Headers {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Headers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (name, value) in &self.headers {
            writeln!(f, "{}: {}", name, value)?;
        }
        Ok(())
    }
}

impl FromIterator<(String, String)> for Headers {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        let mut headers = Headers::new();
        for (name, value) in iter {
            headers.insert(name, value);
        }
        headers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let mut headers = Headers::new();
        headers.insert("Content-Type", "text/html");
        headers.insert("Content-Length", "42");

        assert_eq!(headers.get("Content-Type"), Some("text/html"));
        assert_eq!(headers.get("Content-Length"), Some("42"));
        assert_eq!(headers.get("Missing"), None);
    }

    #[test]
    fn test_case_insensitive() {
        let mut headers = Headers::new();
        headers.insert("Content-Type", "text/html");

        assert_eq!(headers.get("content-type"), Some("text/html"));
        assert_eq!(headers.get("CONTENT-TYPE"), Some("text/html"));
        assert_eq!(headers.get("CoNtEnT-TyPe"), Some("text/html"));
    }

    #[test]
    fn test_multiple_values() {
        let mut headers = Headers::new();
        headers.insert("Set-Cookie", "a=1");
        headers.insert("Set-Cookie", "b=2");
        headers.insert("Set-Cookie", "c=3");

        let values = headers.get_all("Set-Cookie");
        assert_eq!(values.len(), 3);
        assert_eq!(values[0], "a=1");
        assert_eq!(values[1], "b=2");
        assert_eq!(values[2], "c=3");

        assert_eq!(headers.count("Set-Cookie"), 3);
    }

    #[test]
    fn test_get_returns_first() {
        let mut headers = Headers::new();
        headers.insert("X-Custom", "first");
        headers.insert("X-Custom", "second");

        assert_eq!(headers.get("X-Custom"), Some("first"));
    }

    #[test]
    fn test_remove() {
        let mut headers = Headers::new();
        headers.insert("X-Remove", "value1");
        headers.insert("X-Keep", "value2");
        headers.insert("X-Remove", "value3");

        assert_eq!(headers.remove("X-Remove"), 2);
        assert_eq!(headers.get("X-Remove"), None);
        assert_eq!(headers.get("X-Keep"), Some("value2"));
    }

    #[test]
    fn test_contains() {
        let mut headers = Headers::new();
        headers.insert("X-Test", "value");

        assert!(headers.contains("X-Test"));
        assert!(headers.contains("x-test"));
        assert!(!headers.contains("X-Missing"));
    }

    #[test]
    fn test_iter() {
        let mut headers = Headers::new();
        headers.insert("A", "1");
        headers.insert("B", "2");
        headers.insert("C", "3");

        let collected: Vec<_> = headers.iter().collect();
        assert_eq!(collected.len(), 3);
        assert_eq!(collected[0], ("A", "1"));
        assert_eq!(collected[1], ("B", "2"));
        assert_eq!(collected[2], ("C", "3"));
    }

    #[test]
    fn test_parse_header_line() {
        let (name, value) = Headers::parse_header_line("Content-Type: text/html").unwrap();
        assert_eq!(name, "Content-Type");
        assert_eq!(value, "text/html");

        let (name, value) = Headers::parse_header_line("X-Custom:  value  ").unwrap();
        assert_eq!(name, "X-Custom");
        assert_eq!(value, "value");

        assert!(Headers::parse_header_line("Invalid").is_err());
        assert!(Headers::parse_header_line(": value").is_err());
    }

    #[test]
    fn test_max_headers() {
        let mut headers = Headers::new();
        for i in 0..MAX_HEADERS + 10 {
            headers.insert(format!("Header-{}", i), "value");
        }
        // Should be capped at MAX_HEADERS
        assert_eq!(headers.len(), MAX_HEADERS);
    }
}
