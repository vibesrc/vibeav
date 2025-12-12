//! Protocol server traits.
//!
//! Traits for protocol-specific servers that integrate with the core router.
//! Each protocol (RTSP, WebRTC, SRT, etc.) implements these traits.

use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use crate::error::Result;
use crate::router::Router;

/// Trait for protocol servers.
///
/// Each protocol crate (vibeav-rtsp, vibeav-webrtc, etc.) implements this
/// trait to provide a unified interface for the main server binary.
///
/// # Example
///
/// ```rust,ignore
/// use vibeav_core::{Router, ProtocolServer};
/// use vibeav_rtsp::RtspServer;
///
/// let router = Arc::new(Router::new());
/// let rtsp = RtspServer::new(rtsp_config, router.clone());
/// rtsp.start().await?;
/// ```
pub trait ProtocolServer: Send + Sync {
    /// Protocol name (e.g., "rtsp", "webrtc", "srt").
    fn protocol(&self) -> &'static str;

    /// Start the server.
    ///
    /// This should spawn listener tasks and return immediately.
    /// Use `shutdown()` to stop the server.
    fn start(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;

    /// Gracefully shutdown the server.
    fn shutdown(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;

    /// Get the addresses the server is listening on.
    fn listen_addrs(&self) -> Vec<SocketAddr>;

    /// Check if the server is running.
    fn is_running(&self) -> bool;
}

/// Configuration for a protocol listener.
#[derive(Debug, Clone)]
pub struct ListenerConfig {
    /// Bind address.
    pub bind: SocketAddr,
    /// Whether this listener is enabled.
    pub enabled: bool,
}

impl ListenerConfig {
    pub fn new(bind: SocketAddr) -> Self {
        Self { bind, enabled: true }
    }
}

/// Shared server context passed to protocol servers.
#[derive(Clone)]
pub struct ServerContext {
    /// The shared router instance.
    pub router: Arc<Router>,
    /// Server name for headers/identification.
    pub server_name: String,
}

impl ServerContext {
    pub fn new(router: Arc<Router>) -> Self {
        Self {
            router,
            server_name: format!("vibeav/{}", env!("CARGO_PKG_VERSION")),
        }
    }

    pub fn with_server_name(mut self, name: impl Into<String>) -> Self {
        self.server_name = name.into();
        self
    }
}
