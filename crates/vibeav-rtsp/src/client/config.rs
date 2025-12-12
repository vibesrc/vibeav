//! RTSP client configuration.

use std::time::Duration;

/// RTSP client configuration.
#[derive(Debug, Clone)]
pub struct RtspClientConfig {
    /// Source RTSP URL (e.g., "rtsp://192.168.1.69:554/video0").
    pub source_url: String,
    /// Stream path in the router (e.g., "/live/camera1").
    pub stream_path: String,
    /// Connection timeout.
    pub connect_timeout: Duration,
    /// Read timeout for socket operations.
    pub read_timeout: Duration,
    /// Write timeout for socket operations.
    pub write_timeout: Duration,
    /// Prefer TCP interleaved transport.
    pub prefer_tcp: bool,
    /// Reconnect on disconnect.
    pub reconnect: bool,
    /// Reconnect delay.
    pub reconnect_delay: Duration,
    /// Maximum reconnect attempts (0 = infinite).
    pub max_reconnect_attempts: u32,
    /// User agent string.
    pub user_agent: String,
}

impl RtspClientConfig {
    /// Create a new client config with the source URL.
    pub fn new(source_url: impl Into<String>) -> Self {
        let source_url = source_url.into();
        // Default stream path from URL path
        let stream_path = extract_path_from_url(&source_url);

        Self {
            source_url,
            stream_path,
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(10),
            prefer_tcp: true,
            reconnect: true,
            reconnect_delay: Duration::from_secs(5),
            max_reconnect_attempts: 0,
            user_agent: "vibeav-rtsp/0.1".to_string(),
        }
    }

    /// Set the stream path in the router.
    pub fn with_stream_path(mut self, path: impl Into<String>) -> Self {
        self.stream_path = path.into();
        self
    }

    /// Set connection timeout.
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set read timeout.
    pub fn with_read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = timeout;
        self
    }

    /// Set whether to prefer TCP interleaved transport.
    pub fn with_prefer_tcp(mut self, prefer: bool) -> Self {
        self.prefer_tcp = prefer;
        self
    }

    /// Set reconnect behavior.
    pub fn with_reconnect(mut self, reconnect: bool) -> Self {
        self.reconnect = reconnect;
        self
    }

    /// Set reconnect delay.
    pub fn with_reconnect_delay(mut self, delay: Duration) -> Self {
        self.reconnect_delay = delay;
        self
    }

    /// Set maximum reconnect attempts.
    pub fn with_max_reconnect_attempts(mut self, attempts: u32) -> Self {
        self.max_reconnect_attempts = attempts;
        self
    }

    /// Set user agent.
    pub fn with_user_agent(mut self, agent: impl Into<String>) -> Self {
        self.user_agent = agent.into();
        self
    }

    /// Parse the host and port from the source URL.
    pub fn host_port(&self) -> Option<(&str, u16)> {
        parse_rtsp_url(&self.source_url)
    }
}

/// Extract path from RTSP URL.
fn extract_path_from_url(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("rtsp://") {
        if let Some(path_start) = rest.find('/') {
            return rest[path_start..].to_string();
        }
    }
    "/stream".to_string()
}

/// Parse RTSP URL into host and port.
fn parse_rtsp_url(url: &str) -> Option<(&str, u16)> {
    let rest = url.strip_prefix("rtsp://")?;
    let (host_port, _path) = rest.split_once('/').unwrap_or((rest, ""));

    if let Some((host, port_str)) = host_port.split_once(':') {
        let port = port_str.parse().ok()?;
        Some((host, port))
    } else {
        Some((host_port, 554)) // Default RTSP port
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_path() {
        assert_eq!(
            extract_path_from_url("rtsp://192.168.1.69:554/video0"),
            "/video0"
        );
        assert_eq!(
            extract_path_from_url("rtsp://camera.local/live/stream1"),
            "/live/stream1"
        );
    }

    #[test]
    fn test_parse_rtsp_url() {
        assert_eq!(
            parse_rtsp_url("rtsp://192.168.1.69:554/video0"),
            Some(("192.168.1.69", 554))
        );
        assert_eq!(
            parse_rtsp_url("rtsp://camera.local/stream"),
            Some(("camera.local", 554))
        );
        assert_eq!(
            parse_rtsp_url("rtsp://10.0.0.1:8554/live"),
            Some(("10.0.0.1", 8554))
        );
    }

    #[test]
    fn test_config_defaults() {
        let config = RtspClientConfig::new("rtsp://192.168.1.69:554/video0");
        assert_eq!(config.stream_path, "/video0");
        assert!(config.prefer_tcp);
        assert!(config.reconnect);
    }

    #[test]
    fn test_config_with_stream_path() {
        let config = RtspClientConfig::new("rtsp://192.168.1.69:554/video0")
            .with_stream_path("/live/camera1");
        assert_eq!(config.stream_path, "/live/camera1");
    }
}
