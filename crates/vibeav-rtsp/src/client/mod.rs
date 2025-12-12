//! RTSP client for pulling streams from cameras/sources.
//!
//! The RTSP client connects to remote RTSP servers (cameras, media servers)
//! and pulls streams to forward through the router.
//!
//! # Example
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use vibeav_core::Router;
//! use vibeav_rtsp::client::{RtspClient, RtspClientConfig};
//!
//! let router = Arc::new(Router::new());
//! let config = RtspClientConfig::new("rtsp://192.168.1.69:554/video0")
//!     .with_stream_path("/live/camera1");
//!
//! let client = RtspClient::new(config, router);
//! client.start().await?;
//! ```

mod config;
mod connection;
mod session;

pub use config::RtspClientConfig;
pub use connection::RtspClient;
pub use session::ClientSession;
