//! RTSP server implementation.
//!
//! Provides an RTSP server that integrates with vibeav-core's Router.
//!
//! # Supported Operations
//!
//! - **Playback**: DESCRIBE → SETUP → PLAY → TEARDOWN
//! - **Recording**: ANNOUNCE → SETUP → RECORD → TEARDOWN
//!
//! # Example
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use vibeav_core::Router;
//! use vibeav_rtsp::server::{RtspServer, RtspServerConfig};
//!
//! let router = Arc::new(Router::new());
//! let config = RtspServerConfig::default();
//! let server = RtspServer::new(config, router);
//! server.start().await?;
//! ```

mod config;
mod connection;
mod handler;
mod session;

pub use config::RtspServerConfig;

use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};

use vibeav_core::error::Result;
use vibeav_core::server::{ProtocolServer, ServerContext};
use vibeav_core::Router;

use self::connection::Connection;
use self::session::RtspSession;

/// RTSP server.
pub struct RtspServer {
    /// Server configuration.
    config: RtspServerConfig,
    /// Shared server context (router, server name, etc.).
    context: ServerContext,
    /// Active RTSP sessions.
    sessions: Arc<RwLock<HashMap<String, Arc<RtspSession>>>>,
    /// Whether the server is running.
    running: AtomicBool,
    /// Shutdown signal sender.
    shutdown_tx: broadcast::Sender<()>,
    /// Bound addresses after start.
    listen_addrs: RwLock<Vec<SocketAddr>>,
}

impl RtspServer {
    /// Create a new RTSP server.
    pub fn new(config: RtspServerConfig, router: Arc<Router>) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            config,
            context: ServerContext::new(router),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            running: AtomicBool::new(false),
            shutdown_tx,
            listen_addrs: RwLock::new(Vec::new()),
        }
    }

    /// Create with custom server context.
    pub fn with_context(config: RtspServerConfig, context: ServerContext) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            config,
            context,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            running: AtomicBool::new(false),
            shutdown_tx,
            listen_addrs: RwLock::new(Vec::new()),
        }
    }

    /// Get a session by ID.
    pub async fn get_session(&self, session_id: &str) -> Option<Arc<RtspSession>> {
        self.sessions.read().await.get(session_id).cloned()
    }

    /// Create a new session.
    pub async fn create_session(&self) -> Arc<RtspSession> {
        let session = Arc::new(RtspSession::new());
        self.sessions
            .write()
            .await
            .insert(session.id().to_string(), session.clone());
        debug!(session_id = %session.id(), "RTSP session created");
        session
    }

    /// Remove a session.
    pub async fn remove_session(&self, session_id: &str) -> Option<Arc<RtspSession>> {
        let session = self.sessions.write().await.remove(session_id);
        if session.is_some() {
            debug!(session_id = %session_id, "RTSP session removed");
        }
        session
    }

    /// Get the server context.
    pub fn context(&self) -> &ServerContext {
        &self.context
    }

    /// Get the router.
    pub fn router(&self) -> &Arc<Router> {
        &self.context.router
    }

    /// Run the server (blocking).
    async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.config.bind).await.map_err(|e| {
            vibeav_core::error::CoreError::SendFailed(format!("Failed to bind: {}", e))
        })?;

        let local_addr = listener.local_addr().map_err(|e| {
            vibeav_core::error::CoreError::SendFailed(format!("Failed to get local addr: {}", e))
        })?;

        self.listen_addrs.write().await.push(local_addr);
        self.running.store(true, Ordering::SeqCst);

        info!(addr = %local_addr, "RTSP server listening");

        let mut shutdown_rx = self.shutdown_tx.subscribe();

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((socket, peer_addr)) => {
                            debug!(peer = %peer_addr, "New RTSP connection");
                            let conn = Connection::new(
                                socket,
                                peer_addr,
                                self.context.clone(),
                                self.sessions.clone(),
                                self.shutdown_tx.subscribe(),
                            );
                            tokio::spawn(async move {
                                if let Err(e) = conn.run().await {
                                    warn!(peer = %peer_addr, error = %e, "Connection error");
                                }
                            });
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to accept connection");
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("RTSP server shutting down");
                    break;
                }
            }
        }

        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }
}

impl ProtocolServer for RtspServer {
    fn protocol(&self) -> &'static str {
        "rtsp"
    }

    fn start(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(self.run())
    }

    fn shutdown(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            let _ = self.shutdown_tx.send(());
        })
    }

    fn listen_addrs(&self) -> Vec<SocketAddr> {
        // This is a sync method, so we can't await.
        // Return empty if not started yet.
        Vec::new()
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}
