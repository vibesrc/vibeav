//! RTSP errors.

use thiserror::Error;

/// RTSP protocol errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RtspError {
    /// Invalid RTSP version.
    #[error("invalid RTSP version: {0}")]
    InvalidVersion(String),

    /// Invalid method.
    #[error("invalid method: {0}")]
    InvalidMethod(String),

    /// Invalid status code.
    #[error("invalid status code: {0}")]
    InvalidStatusCode(u16),

    /// Invalid request line.
    #[error("invalid request line: {0}")]
    InvalidRequestLine(String),

    /// Invalid request.
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// Invalid status line.
    #[error("invalid status line: {0}")]
    InvalidStatusLine(String),

    /// Invalid header.
    #[error("invalid header: {0}")]
    InvalidHeader(String),

    /// Missing required header.
    #[error("missing required header: {0}")]
    MissingHeader(String),

    /// Invalid CSeq.
    #[error("invalid CSeq: {0}")]
    InvalidCSeq(String),

    /// Invalid transport header.
    #[error("invalid transport header: {0}")]
    InvalidTransport(String),

    /// Unsupported transport.
    #[error("unsupported transport: {0}")]
    UnsupportedTransport(String),

    /// Invalid content length.
    #[error("invalid content length: {0}")]
    InvalidContentLength(String),

    /// Message too large.
    #[error("message too large: {actual} > {max}")]
    MessageTooLarge { actual: usize, max: usize },

    /// Incomplete message.
    #[error("incomplete message")]
    Incomplete,

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(String),

    /// Session error.
    #[error("session error: {0}")]
    Session(String),

    /// Session not found.
    #[error("session not found: {0}")]
    SessionNotFound(String),

    /// Resource not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Method not allowed.
    #[error("method not allowed: {0}")]
    MethodNotAllowed(String),

    /// Invalid state for operation.
    #[error("invalid state: {0}")]
    InvalidState(String),

    /// Protocol error.
    #[error("protocol error: {0}")]
    Protocol(String),
}

/// RTSP result type.
pub type Result<T> = std::result::Result<T, RtspError>;
