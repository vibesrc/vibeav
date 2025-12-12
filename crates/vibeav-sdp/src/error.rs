//! SDP parsing errors.

use thiserror::Error;

/// SDP parsing errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SdpError {
    /// Missing required version line.
    #[error("missing version line (v=)")]
    MissingVersion,

    /// Invalid version number.
    #[error("invalid version: expected 0, got {0}")]
    InvalidVersion(u8),

    /// Missing required origin line.
    #[error("missing origin line (o=)")]
    MissingOrigin,

    /// Invalid origin line format.
    #[error("invalid origin line: {0}")]
    InvalidOrigin(String),

    /// Missing required session name.
    #[error("missing session name line (s=)")]
    MissingSessionName,

    /// Invalid timing line format.
    #[error("invalid timing line: {0}")]
    InvalidTiming(String),

    /// Invalid media line format.
    #[error("invalid media line: {0}")]
    InvalidMedia(String),

    /// Invalid connection line format.
    #[error("invalid connection line: {0}")]
    InvalidConnection(String),

    /// Invalid rtpmap attribute.
    #[error("invalid rtpmap: {0}")]
    InvalidRtpmap(String),

    /// Invalid fmtp attribute.
    #[error("invalid fmtp: {0}")]
    InvalidFmtp(String),

    /// Invalid attribute line.
    #[error("invalid attribute line: {0}")]
    InvalidAttribute(String),

    /// Invalid line format.
    #[error("invalid line format at line {line}: {message}")]
    InvalidLine { line: usize, message: String },

    /// Empty SDP.
    #[error("empty SDP")]
    Empty,
}
