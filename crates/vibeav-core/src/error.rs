//! Core router errors.

use thiserror::Error;

/// Core router errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// Stream not found.
    #[error("stream not found: {0}")]
    StreamNotFound(String),

    /// Stream already exists.
    #[error("stream already exists: {0}")]
    StreamExists(String),

    /// Session not found.
    #[error("session not found: {0}")]
    SessionNotFound(String),

    /// Session already exists.
    #[error("session already exists: {0}")]
    SessionExists(String),

    /// Sink not found.
    #[error("sink not found: {0}")]
    SinkNotFound(String),

    /// Track not found.
    #[error("track not found: {0}")]
    TrackNotFound(String),

    /// Source already exists for this stream.
    #[error("stream already has a source: {0}")]
    SourceExists(String),

    /// No source for this stream.
    #[error("stream has no source: {0}")]
    NoSource(String),

    /// Attachment not found.
    #[error("attachment not found: {0}")]
    AttachmentNotFound(String),

    /// Queue is full.
    #[error("output queue full")]
    QueueFull,

    /// Send failed.
    #[error("send failed: {0}")]
    SendFailed(String),

    /// Channel closed.
    #[error("channel closed")]
    ChannelClosed,
}

/// Result type for core operations.
pub type Result<T> = std::result::Result<T, CoreError>;
