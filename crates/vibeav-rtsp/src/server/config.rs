//! RTSP server configuration.

use std::net::SocketAddr;
use std::time::Duration;

/// RTSP server configuration.
#[derive(Debug, Clone)]
pub struct RtspServerConfig {
    /// Bind address.
    pub bind: SocketAddr,
    /// Session timeout.
    pub session_timeout: Duration,
    /// Read timeout for connections.
    pub read_timeout: Duration,
    /// Write timeout for connections.
    pub write_timeout: Duration,
    /// Maximum request size in bytes.
    pub max_request_size: usize,
    /// Whether to allow ANNOUNCE/RECORD (publishing).
    pub allow_publish: bool,
    /// Whether to allow DESCRIBE/PLAY (playback / sink).
    pub allow_play: bool,
}

impl Default for RtspServerConfig {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0:554".parse().unwrap(),
            session_timeout: Duration::from_secs(60),
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(30),
            max_request_size: 64 * 1024, // 64 KB
            allow_publish: true,
            allow_play: true,
        }
    }
}

impl RtspServerConfig {
    /// Create a new config with custom bind address.
    pub fn with_bind(mut self, bind: SocketAddr) -> Self {
        self.bind = bind;
        self
    }

    /// Set session timeout.
    pub fn with_session_timeout(mut self, timeout: Duration) -> Self {
        self.session_timeout = timeout;
        self
    }

    /// Enable/disable publishing (ANNOUNCE/RECORD).
    pub fn with_publish(mut self, allow: bool) -> Self {
        self.allow_publish = allow;
        self
    }

    /// Enable/disable playback (DESCRIBE/PLAY).
    pub fn with_play(mut self, allow: bool) -> Self {
        self.allow_play = allow;
        self
    }
}
