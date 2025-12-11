//! RTSP status codes.

use std::fmt;

/// RTSP status code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatusCode(pub u16);

impl StatusCode {
    // 1xx - Informational
    pub const CONTINUE: StatusCode = StatusCode(100);

    // 2xx - Success
    pub const OK: StatusCode = StatusCode(200);
    pub const CREATED: StatusCode = StatusCode(201);
    pub const LOW_ON_STORAGE: StatusCode = StatusCode(250);

    // 3xx - Redirection
    pub const MULTIPLE_CHOICES: StatusCode = StatusCode(300);
    pub const MOVED_PERMANENTLY: StatusCode = StatusCode(301);
    pub const MOVED_TEMPORARILY: StatusCode = StatusCode(302);
    pub const SEE_OTHER: StatusCode = StatusCode(303);
    pub const NOT_MODIFIED: StatusCode = StatusCode(304);
    pub const USE_PROXY: StatusCode = StatusCode(305);

    // 4xx - Client Error
    pub const BAD_REQUEST: StatusCode = StatusCode(400);
    pub const UNAUTHORIZED: StatusCode = StatusCode(401);
    pub const PAYMENT_REQUIRED: StatusCode = StatusCode(402);
    pub const FORBIDDEN: StatusCode = StatusCode(403);
    pub const NOT_FOUND: StatusCode = StatusCode(404);
    pub const METHOD_NOT_ALLOWED: StatusCode = StatusCode(405);
    pub const NOT_ACCEPTABLE: StatusCode = StatusCode(406);
    pub const PROXY_AUTH_REQUIRED: StatusCode = StatusCode(407);
    pub const REQUEST_TIMEOUT: StatusCode = StatusCode(408);
    pub const GONE: StatusCode = StatusCode(410);
    pub const LENGTH_REQUIRED: StatusCode = StatusCode(411);
    pub const PRECONDITION_FAILED: StatusCode = StatusCode(412);
    pub const REQUEST_ENTITY_TOO_LARGE: StatusCode = StatusCode(413);
    pub const REQUEST_URI_TOO_LARGE: StatusCode = StatusCode(414);
    pub const UNSUPPORTED_MEDIA_TYPE: StatusCode = StatusCode(415);
    pub const PARAMETER_NOT_UNDERSTOOD: StatusCode = StatusCode(451);
    pub const CONFERENCE_NOT_FOUND: StatusCode = StatusCode(452);
    pub const NOT_ENOUGH_BANDWIDTH: StatusCode = StatusCode(453);
    pub const SESSION_NOT_FOUND: StatusCode = StatusCode(454);
    pub const METHOD_NOT_VALID: StatusCode = StatusCode(455);
    pub const HEADER_FIELD_NOT_VALID: StatusCode = StatusCode(456);
    pub const INVALID_RANGE: StatusCode = StatusCode(457);
    pub const PARAMETER_READ_ONLY: StatusCode = StatusCode(458);
    pub const AGGREGATE_OPERATION_NOT_ALLOWED: StatusCode = StatusCode(459);
    pub const ONLY_AGGREGATE_OPERATION_ALLOWED: StatusCode = StatusCode(460);
    pub const UNSUPPORTED_TRANSPORT: StatusCode = StatusCode(461);
    pub const DESTINATION_UNREACHABLE: StatusCode = StatusCode(462);

    // 5xx - Server Error
    pub const INTERNAL_SERVER_ERROR: StatusCode = StatusCode(500);
    pub const NOT_IMPLEMENTED: StatusCode = StatusCode(501);
    pub const BAD_GATEWAY: StatusCode = StatusCode(502);
    pub const SERVICE_UNAVAILABLE: StatusCode = StatusCode(503);
    pub const GATEWAY_TIMEOUT: StatusCode = StatusCode(504);
    pub const RTSP_VERSION_NOT_SUPPORTED: StatusCode = StatusCode(505);
    pub const OPTION_NOT_SUPPORTED: StatusCode = StatusCode(551);

    /// Get the numeric status code.
    pub fn as_u16(&self) -> u16 {
        self.0
    }

    /// Get the reason phrase for this status code.
    pub fn reason(&self) -> &'static str {
        match self.0 {
            100 => "Continue",
            200 => "OK",
            201 => "Created",
            250 => "Low on Storage Space",
            300 => "Multiple Choices",
            301 => "Moved Permanently",
            302 => "Moved Temporarily",
            303 => "See Other",
            304 => "Not Modified",
            305 => "Use Proxy",
            400 => "Bad Request",
            401 => "Unauthorized",
            402 => "Payment Required",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            406 => "Not Acceptable",
            407 => "Proxy Authentication Required",
            408 => "Request Timeout",
            410 => "Gone",
            411 => "Length Required",
            412 => "Precondition Failed",
            413 => "Request Entity Too Large",
            414 => "Request-URI Too Large",
            415 => "Unsupported Media Type",
            451 => "Parameter Not Understood",
            452 => "Conference Not Found",
            453 => "Not Enough Bandwidth",
            454 => "Session Not Found",
            455 => "Method Not Valid in This State",
            456 => "Header Field Not Valid for Resource",
            457 => "Invalid Range",
            458 => "Parameter Is Read-Only",
            459 => "Aggregate Operation Not Allowed",
            460 => "Only Aggregate Operation Allowed",
            461 => "Unsupported Transport",
            462 => "Destination Unreachable",
            500 => "Internal Server Error",
            501 => "Not Implemented",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            504 => "Gateway Timeout",
            505 => "RTSP Version Not Supported",
            551 => "Option Not Supported",
            _ => "Unknown",
        }
    }

    /// Check if this is a success status (2xx).
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.0)
    }

    /// Check if this is a redirection status (3xx).
    pub fn is_redirection(&self) -> bool {
        (300..400).contains(&self.0)
    }

    /// Check if this is a client error (4xx).
    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.0)
    }

    /// Check if this is a server error (5xx).
    pub fn is_server_error(&self) -> bool {
        (500..600).contains(&self.0)
    }

    /// Check if this is an error (4xx or 5xx).
    pub fn is_error(&self) -> bool {
        self.is_client_error() || self.is_server_error()
    }
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u16> for StatusCode {
    fn from(code: u16) -> Self {
        StatusCode(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_code_success() {
        assert!(StatusCode::OK.is_success());
        assert!(StatusCode::CREATED.is_success());
        assert!(!StatusCode::BAD_REQUEST.is_success());
    }

    #[test]
    fn test_status_code_error() {
        assert!(StatusCode::BAD_REQUEST.is_client_error());
        assert!(StatusCode::NOT_FOUND.is_client_error());
        assert!(StatusCode::INTERNAL_SERVER_ERROR.is_server_error());
        assert!(StatusCode::NOT_FOUND.is_error());
        assert!(!StatusCode::OK.is_error());
    }

    #[test]
    fn test_status_code_reason() {
        assert_eq!(StatusCode::OK.reason(), "OK");
        assert_eq!(StatusCode::NOT_FOUND.reason(), "Not Found");
        assert_eq!(StatusCode::UNSUPPORTED_TRANSPORT.reason(), "Unsupported Transport");
    }
}
