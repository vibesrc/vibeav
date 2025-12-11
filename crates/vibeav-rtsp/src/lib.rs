//! RTSP client and server for vibeav.
//!
//! This crate provides RTSP protocol implementation including:
//! - Request and response parsing/serialization
//! - Transport header parsing
//! - Method definitions and status codes
//! - RTSP server with core Router integration
//!
//! # Server Example
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
//!
//! # Message Example
//!
//! ```rust
//! use vibeav_rtsp::message::{Request, Response, Method, StatusCode};
//! use vibeav_rtsp::transport::Transport;
//!
//! // Create a DESCRIBE request
//! let req = Request::describe("rtsp://example.com/movie").with_cseq(1);
//!
//! // Create an OK response
//! let resp = Response::ok()
//!     .with_cseq(1)
//!     .with_public(&["DESCRIBE", "SETUP", "PLAY", "TEARDOWN"]);
//!
//! // Parse transport header
//! let transport = Transport::parse("RTP/AVP/TCP;unicast;interleaved=0-1").unwrap();
//! assert!(transport.is_interleaved());
//! ```

pub mod client;
pub mod error;
pub mod message;
pub mod server;
pub mod transport;

pub use client::{ClientSession, RtspClient, RtspClientConfig};
pub use error::{RtspError, Result};
pub use message::{Headers, Method, Request, Response, StatusCode, RTSP_VERSION};
pub use server::{RtspServer, RtspServerConfig};
pub use transport::{
    CastMode, LowerTransport, Transport, TransportMode, TransportProfile, TransportProtocol,
};
