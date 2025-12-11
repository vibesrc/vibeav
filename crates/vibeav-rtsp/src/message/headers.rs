//! RTSP headers.

use std::collections::HashMap;
use std::fmt;

/// RTSP headers collection.
#[derive(Debug, Clone, Default)]
pub struct Headers {
    inner: HashMap<String, String>,
}

impl Headers {
    /// Create empty headers.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a header (case-insensitive key).
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.inner.insert(key.into().to_lowercase(), value.into());
    }

    /// Get a header value (case-insensitive).
    pub fn get(&self, key: &str) -> Option<&str> {
        self.inner.get(&key.to_lowercase()).map(|s| s.as_str())
    }

    /// Remove a header (case-insensitive).
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.inner.remove(&key.to_lowercase())
    }

    /// Check if header exists (case-insensitive).
    pub fn contains(&self, key: &str) -> bool {
        self.inner.contains_key(&key.to_lowercase())
    }

    /// Get number of headers.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Iterate over headers.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.inner.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    // Common header getters

    /// Get CSeq header.
    pub fn cseq(&self) -> Option<u32> {
        self.get("cseq").and_then(|v| v.parse().ok())
    }

    /// Get Session header.
    pub fn session(&self) -> Option<&str> {
        self.get("session").map(|s| {
            // Session may include timeout: "12345678;timeout=60"
            s.split(';').next().unwrap_or(s)
        })
    }

    /// Get Session timeout.
    pub fn session_timeout(&self) -> Option<u32> {
        self.get("session").and_then(|s| {
            s.split(';')
                .find(|p| p.trim().starts_with("timeout="))
                .and_then(|p| p.trim().strip_prefix("timeout="))
                .and_then(|v| v.parse().ok())
        })
    }

    /// Get Content-Length header.
    pub fn content_length(&self) -> Option<usize> {
        self.get("content-length").and_then(|v| v.parse().ok())
    }

    /// Get Content-Type header.
    pub fn content_type(&self) -> Option<&str> {
        self.get("content-type")
    }

    /// Get Content-Base header.
    pub fn content_base(&self) -> Option<&str> {
        self.get("content-base")
    }

    /// Get Transport header.
    pub fn transport(&self) -> Option<&str> {
        self.get("transport")
    }

    /// Get Public header (comma-separated methods).
    pub fn public_methods(&self) -> Option<Vec<&str>> {
        self.get("public").map(|v| v.split(',').map(|s| s.trim()).collect())
    }

    /// Get Accept header.
    pub fn accept(&self) -> Option<&str> {
        self.get("accept")
    }

    /// Get Range header.
    pub fn range(&self) -> Option<&str> {
        self.get("range")
    }

    /// Get RTP-Info header.
    pub fn rtp_info(&self) -> Option<&str> {
        self.get("rtp-info")
    }

    /// Get Server header.
    pub fn server(&self) -> Option<&str> {
        self.get("server")
    }

    /// Get User-Agent header.
    pub fn user_agent(&self) -> Option<&str> {
        self.get("user-agent")
    }
}

impl fmt::Display for Headers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (key, value) in &self.inner {
            // Capitalize header names for output
            let key_display = key
                .split('-')
                .map(|part| {
                    let mut chars: Vec<char> = part.chars().collect();
                    if let Some(first) = chars.first_mut() {
                        *first = first.to_uppercase().next().unwrap_or(*first);
                    }
                    chars.into_iter().collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("-");
            // Use CRLF for RTSP compliance
            write!(f, "{}: {}\r\n", key_display, value)?;
        }
        Ok(())
    }
}

impl<K, V> FromIterator<(K, V)> for Headers
where
    K: Into<String>,
    V: Into<String>,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut headers = Headers::new();
        for (k, v) in iter {
            headers.insert(k, v);
        }
        headers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_headers_case_insensitive() {
        let mut headers = Headers::new();
        headers.insert("Content-Type", "application/sdp");

        assert_eq!(headers.get("content-type"), Some("application/sdp"));
        assert_eq!(headers.get("Content-Type"), Some("application/sdp"));
        assert_eq!(headers.get("CONTENT-TYPE"), Some("application/sdp"));
    }

    #[test]
    fn test_headers_cseq() {
        let mut headers = Headers::new();
        headers.insert("CSeq", "123");
        assert_eq!(headers.cseq(), Some(123));
    }

    #[test]
    fn test_headers_session() {
        let mut headers = Headers::new();
        headers.insert("Session", "12345678");
        assert_eq!(headers.session(), Some("12345678"));

        headers.insert("Session", "12345678;timeout=60");
        assert_eq!(headers.session(), Some("12345678"));
        assert_eq!(headers.session_timeout(), Some(60));
    }

    #[test]
    fn test_headers_public_methods() {
        let mut headers = Headers::new();
        headers.insert("Public", "DESCRIBE, SETUP, PLAY, TEARDOWN");

        let methods = headers.public_methods().unwrap();
        assert_eq!(methods, vec!["DESCRIBE", "SETUP", "PLAY", "TEARDOWN"]);
    }
}
