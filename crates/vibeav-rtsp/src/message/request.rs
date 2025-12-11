//! RTSP request.

use std::fmt;

use crate::error::{RtspError, Result};
use crate::message::headers::Headers;
use crate::message::method::Method;

/// RTSP version.
pub const RTSP_VERSION: &str = "RTSP/1.0";

/// RTSP request.
#[derive(Debug, Clone)]
pub struct Request {
    /// Request method.
    pub method: Method,
    /// Request URI.
    pub uri: String,
    /// RTSP version (always "RTSP/1.0").
    pub version: String,
    /// Request headers.
    pub headers: Headers,
    /// Request body (e.g., SDP for ANNOUNCE).
    pub body: Option<Vec<u8>>,
}

impl Request {
    /// Create a new request.
    pub fn new(method: Method, uri: impl Into<String>) -> Self {
        Self {
            method,
            uri: uri.into(),
            version: RTSP_VERSION.to_string(),
            headers: Headers::new(),
            body: None,
        }
    }

    /// Create an OPTIONS request.
    pub fn options(uri: impl Into<String>) -> Self {
        Self::new(Method::Options, uri)
    }

    /// Create a DESCRIBE request.
    pub fn describe(uri: impl Into<String>) -> Self {
        let mut req = Self::new(Method::Describe, uri);
        req.headers.insert("Accept", "application/sdp");
        req
    }

    /// Create an ANNOUNCE request.
    pub fn announce(uri: impl Into<String>, sdp: impl Into<Vec<u8>>) -> Self {
        let body = sdp.into();
        let mut req = Self::new(Method::Announce, uri);
        req.headers.insert("Content-Type", "application/sdp");
        req.headers.insert("Content-Length", body.len().to_string());
        req.body = Some(body);
        req
    }

    /// Create a SETUP request.
    pub fn setup(uri: impl Into<String>) -> Self {
        Self::new(Method::Setup, uri)
    }

    /// Create a SETUP request with transport.
    pub fn setup_with_transport(uri: impl Into<String>, transport: impl Into<String>) -> Self {
        let mut req = Self::new(Method::Setup, uri);
        req.headers.insert("Transport", transport);
        req
    }

    /// Create a PLAY request.
    pub fn play(uri: impl Into<String>) -> Self {
        Self::new(Method::Play, uri)
    }

    /// Create a PLAY request with session.
    pub fn play_with_session(uri: impl Into<String>, session: impl Into<String>) -> Self {
        let mut req = Self::new(Method::Play, uri);
        req.headers.insert("Session", session);
        req
    }

    /// Create a PAUSE request.
    pub fn pause(uri: impl Into<String>) -> Self {
        Self::new(Method::Pause, uri)
    }

    /// Create a PAUSE request with session.
    pub fn pause_with_session(uri: impl Into<String>, session: impl Into<String>) -> Self {
        let mut req = Self::new(Method::Pause, uri);
        req.headers.insert("Session", session);
        req
    }

    /// Create a RECORD request.
    pub fn record(uri: impl Into<String>) -> Self {
        Self::new(Method::Record, uri)
    }

    /// Create a RECORD request with session.
    pub fn record_with_session(uri: impl Into<String>, session: impl Into<String>) -> Self {
        let mut req = Self::new(Method::Record, uri);
        req.headers.insert("Session", session);
        req
    }

    /// Create a TEARDOWN request.
    pub fn teardown(uri: impl Into<String>) -> Self {
        Self::new(Method::Teardown, uri)
    }

    /// Create a TEARDOWN request with session.
    pub fn teardown_with_session(uri: impl Into<String>, session: impl Into<String>) -> Self {
        let mut req = Self::new(Method::Teardown, uri);
        req.headers.insert("Session", session);
        req
    }

    /// Add a header.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Set CSeq header.
    pub fn with_cseq(mut self, cseq: u32) -> Self {
        self.headers.insert("CSeq", cseq.to_string());
        self
    }

    /// Set Session header.
    pub fn with_session(mut self, session: impl Into<String>) -> Self {
        self.headers.insert("Session", session);
        self
    }

    /// Set User-Agent header.
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.headers.insert("User-Agent", user_agent);
        self
    }

    /// Get CSeq from headers.
    pub fn cseq(&self) -> Option<u32> {
        self.headers.cseq()
    }

    /// Get Session from headers.
    pub fn session(&self) -> Option<&str> {
        self.headers.session()
    }

    /// Parse a request from bytes.
    pub fn parse(data: &[u8]) -> Result<(Self, usize)> {
        let text = std::str::from_utf8(data)
            .map_err(|e| RtspError::InvalidRequestLine(e.to_string()))?;

        // Find end of headers
        let header_end = text
            .find("\r\n\r\n")
            .ok_or(RtspError::Incomplete)?;

        let header_section = &text[..header_end];
        let mut lines = header_section.lines();

        // Parse request line
        let request_line = lines.next().ok_or(RtspError::Incomplete)?;
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() != 3 {
            return Err(RtspError::InvalidRequestLine(request_line.to_string()));
        }

        let method: Method = parts[0].parse()?;
        let uri = parts[1].to_string();
        let version = parts[2].to_string();

        if !version.starts_with("RTSP/") {
            return Err(RtspError::InvalidVersion(version));
        }

        // Parse headers
        let mut headers = Headers::new();
        for line in lines {
            if line.is_empty() {
                continue;
            }
            if let Some((key, value)) = line.split_once(':') {
                headers.insert(key.trim(), value.trim());
            }
        }

        // Check for body
        let mut consumed = header_end + 4; // +4 for \r\n\r\n
        let body = if let Some(content_length) = headers.content_length() {
            if data.len() < consumed + content_length {
                return Err(RtspError::Incomplete);
            }
            let body_data = data[consumed..consumed + content_length].to_vec();
            consumed += content_length;
            Some(body_data)
        } else {
            None
        };

        Ok((
            Self {
                method,
                uri,
                version,
                headers,
                body,
            },
            consumed,
        ))
    }

    /// Serialize request to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // Request line
        buf.extend(format!("{} {} {}\r\n", self.method, self.uri, self.version).as_bytes());

        // Headers
        buf.extend(self.headers.to_string().as_bytes());

        // Empty line
        buf.extend(b"\r\n");

        // Body
        if let Some(body) = &self.body {
            buf.extend(body);
        }

        buf
    }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.method, self.uri, self.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_parse_simple() {
        let data = b"OPTIONS rtsp://example.com/movie RTSP/1.0\r\n\
                     CSeq: 1\r\n\
                     \r\n";

        let (req, consumed) = Request::parse(data).unwrap();
        assert_eq!(req.method, Method::Options);
        assert_eq!(req.uri, "rtsp://example.com/movie");
        assert_eq!(req.cseq(), Some(1));
        assert_eq!(consumed, data.len());
    }

    #[test]
    fn test_request_parse_with_body() {
        let body = b"v=0\r\no=- 1234 1234 IN IP4 127.0.0.1\r\ns=Test\r\n";
        let data = format!(
            "ANNOUNCE rtsp://example.com/stream RTSP/1.0\r\n\
             CSeq: 3\r\n\
             Content-Type: application/sdp\r\n\
             Content-Length: {}\r\n\
             \r\n{}",
            body.len(),
            std::str::from_utf8(body).unwrap()
        );

        let (req, _) = Request::parse(data.as_bytes()).unwrap();
        assert_eq!(req.method, Method::Announce);
        assert_eq!(req.body, Some(body.to_vec()));
    }

    #[test]
    fn test_request_serialize() {
        let req = Request::describe("rtsp://example.com/movie").with_cseq(1);
        let bytes = req.to_bytes();
        let text = String::from_utf8(bytes).unwrap();

        assert!(text.contains("DESCRIBE rtsp://example.com/movie RTSP/1.0"));
        assert!(text.contains("Cseq: 1"));
        assert!(text.contains("Accept: application/sdp"));
    }

    #[test]
    fn test_request_incomplete() {
        let data = b"OPTIONS rtsp://example.com/movie RTSP/1.0\r\n\
                     CSeq: 1\r\n";

        assert!(matches!(Request::parse(data), Err(RtspError::Incomplete)));
    }
}
