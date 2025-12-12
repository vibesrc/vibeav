//! RTSP response.

use std::fmt;

use crate::error::{RtspError, Result};
use crate::message::headers::Headers;
use crate::message::request::RTSP_VERSION;
use crate::message::status::StatusCode;

/// RTSP response.
#[derive(Debug, Clone)]
pub struct Response {
    /// RTSP version.
    pub version: String,
    /// Status code.
    pub status: StatusCode,
    /// Reason phrase.
    pub reason: String,
    /// Response headers.
    pub headers: Headers,
    /// Response body.
    pub body: Option<Vec<u8>>,
}

impl Response {
    /// Create a new response.
    pub fn new(status: StatusCode) -> Self {
        Self {
            version: RTSP_VERSION.to_string(),
            status,
            reason: status.reason().to_string(),
            headers: Headers::new(),
            body: None,
        }
    }

    /// Create an OK response.
    pub fn ok() -> Self {
        Self::new(StatusCode::OK)
    }

    /// Create a Not Found response.
    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND)
    }

    /// Create a Bad Request response.
    pub fn bad_request() -> Self {
        Self::new(StatusCode::BAD_REQUEST)
    }

    /// Create an Internal Server Error response.
    pub fn internal_error() -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR)
    }

    /// Create an Unsupported Transport response.
    pub fn unsupported_transport() -> Self {
        Self::new(StatusCode::UNSUPPORTED_TRANSPORT)
    }

    /// Create a Session Not Found response.
    pub fn session_not_found() -> Self {
        Self::new(StatusCode::SESSION_NOT_FOUND)
    }

    /// Create a Method Not Allowed response.
    pub fn method_not_allowed() -> Self {
        Self::new(StatusCode::METHOD_NOT_ALLOWED)
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

    /// Set Session header with timeout.
    pub fn with_session_timeout(mut self, session: impl Into<String>, timeout: u32) -> Self {
        self.headers
            .insert("Session", format!("{};timeout={}", session.into(), timeout));
        self
    }

    /// Set Transport header.
    pub fn with_transport(mut self, transport: impl Into<String>) -> Self {
        self.headers.insert("Transport", transport);
        self
    }

    /// Set Public header.
    pub fn with_public(mut self, methods: &[&str]) -> Self {
        self.headers.insert("Public", methods.join(", "));
        self
    }

    /// Set Content-Type header.
    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.headers.insert("Content-Type", content_type);
        self
    }

    /// Set Content-Base header.
    pub fn with_content_base(mut self, content_base: impl Into<String>) -> Self {
        self.headers.insert("Content-Base", content_base);
        self
    }

    /// Set body.
    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        let body = body.into();
        self.headers.insert("Content-Length", body.len().to_string());
        self.body = Some(body);
        self
    }

    /// Set SDP body.
    pub fn with_sdp(self, sdp: impl Into<Vec<u8>>) -> Self {
        self.with_content_type("application/sdp").with_body(sdp)
    }

    /// Set Server header.
    pub fn with_server(mut self, server: impl Into<String>) -> Self {
        self.headers.insert("Server", server);
        self
    }

    /// Set RTP-Info header.
    pub fn with_rtp_info(mut self, rtp_info: impl Into<String>) -> Self {
        self.headers.insert("RTP-Info", rtp_info);
        self
    }

    /// Set Range header.
    pub fn with_range(mut self, range: impl Into<String>) -> Self {
        self.headers.insert("Range", range);
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

    /// Check if response is success.
    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    /// Check if response is error.
    pub fn is_error(&self) -> bool {
        self.status.is_error()
    }

    /// Parse a response from bytes.
    pub fn parse(data: &[u8]) -> Result<(Self, usize)> {
        let text = std::str::from_utf8(data)
            .map_err(|e| RtspError::InvalidStatusLine(e.to_string()))?;

        // Find end of headers
        let header_end = text.find("\r\n\r\n").ok_or(RtspError::Incomplete)?;

        let header_section = &text[..header_end];
        let mut lines = header_section.lines();

        // Parse status line: RTSP/1.0 200 OK
        let status_line = lines.next().ok_or(RtspError::Incomplete)?;
        let mut parts = status_line.splitn(3, ' ');

        let version = parts
            .next()
            .ok_or_else(|| RtspError::InvalidStatusLine(status_line.to_string()))?
            .to_string();

        if !version.starts_with("RTSP/") {
            return Err(RtspError::InvalidVersion(version));
        }

        let status_code: u16 = parts
            .next()
            .ok_or_else(|| RtspError::InvalidStatusLine(status_line.to_string()))?
            .parse()
            .map_err(|_| RtspError::InvalidStatusLine(status_line.to_string()))?;

        let reason = parts.next().unwrap_or("").to_string();

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
        let mut consumed = header_end + 4;
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
                version,
                status: StatusCode(status_code),
                reason,
                headers,
                body,
            },
            consumed,
        ))
    }

    /// Serialize response to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // Status line
        buf.extend(
            format!("{} {} {}\r\n", self.version, self.status, self.reason).as_bytes(),
        );

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

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.version, self.status, self.reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_parse_simple() {
        let data = b"RTSP/1.0 200 OK\r\n\
                     CSeq: 1\r\n\
                     Public: DESCRIBE, SETUP, PLAY\r\n\
                     \r\n";

        let (resp, consumed) = Response::parse(data).unwrap();
        assert_eq!(resp.status, StatusCode::OK);
        assert_eq!(resp.cseq(), Some(1));
        assert_eq!(consumed, data.len());
    }

    #[test]
    fn test_response_parse_with_body() {
        let body = b"v=0\r\no=- 1234 1234 IN IP4 127.0.0.1\r\ns=Test\r\n";
        let data = format!(
            "RTSP/1.0 200 OK\r\n\
             CSeq: 2\r\n\
             Content-Type: application/sdp\r\n\
             Content-Length: {}\r\n\
             \r\n{}",
            body.len(),
            std::str::from_utf8(body).unwrap()
        );

        let (resp, _) = Response::parse(data.as_bytes()).unwrap();
        assert!(resp.is_success());
        assert_eq!(resp.body, Some(body.to_vec()));
    }

    #[test]
    fn test_response_serialize() {
        let resp = Response::ok()
            .with_cseq(1)
            .with_public(&["DESCRIBE", "SETUP", "PLAY", "TEARDOWN"]);

        let bytes = resp.to_bytes();
        let text = String::from_utf8(bytes).unwrap();

        assert!(text.contains("RTSP/1.0 200 OK"));
        assert!(text.contains("Cseq: 1"));
        assert!(text.contains("Public: DESCRIBE, SETUP, PLAY, TEARDOWN"));
    }

    #[test]
    fn test_response_with_session_timeout() {
        let resp = Response::ok()
            .with_cseq(1)
            .with_session_timeout("12345678", 60);

        let session = resp.headers.get("session").unwrap();
        assert!(session.contains("12345678"));
        assert!(session.contains("timeout=60"));
    }
}
