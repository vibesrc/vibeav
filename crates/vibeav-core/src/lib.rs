//! Core router engine for vibeav streaming server.
//!
//! This crate provides the central routing logic for media streams.
//! It handles:
//! - Stream management (create, remove, list)
//! - Session management (client connections)
//! - Source/Sink management (who sends and receives media)
//! - Zero-copy packet forwarding
//!
//! # Terminology
//!
//! - **Stream**: A media source (e.g., camera feed)
//! - **Source**: Entity publishing media to a stream
//! - **Sink**: Entity receiving media from a stream
//! - **Attachment**: Binding between a track and a sink
//! - **Session**: Client connection with state
//!
//! # Example
//!
//! ```rust,no_run
//! use vibeav_core::router::Router;
//! use bytes::Bytes;
//!
//! #[tokio::main]
//! async fn main() {
//!     let router = Router::new();
//!
//!     // Create a stream
//!     let stream = router.create_stream("camera1").await.unwrap();
//!
//!     // Add a sink to receive media
//!     let mut rx = router.add_sink("camera1", "client1", None).await.unwrap();
//!
//!     // Set source
//!     router.set_source("camera1", "source1").await.unwrap();
//!
//!     // Forward a packet (normally from RTP parser)
//!     let rtp_packet = Bytes::from(vec![
//!         0x80, 0x60, 0x00, 0x01, // V=2, P=0, X=0, CC=0, M=0, PT=96, Seq=1
//!         0x00, 0x00, 0x00, 0x00, // Timestamp
//!         0x00, 0x00, 0x00, 0x01, // SSRC
//!         0x00, 0x00, 0x00, 0x00, // Payload
//!     ]);
//!     router.on_rtp("camera1", rtp_packet).await.unwrap();
//!
//!     // Receive the packet
//!     if let Some(packet) = rx.recv().await {
//!         println!("Received packet seq={}", packet.sequence);
//!     }
//! }
//! ```

pub mod attachment;
pub mod error;
pub mod router;
pub mod server;
pub mod session;
pub mod sink;
pub mod source;
pub mod stream;

pub use attachment::{Attachment, AttachmentId};
pub use error::{CoreError, Result};
pub use router::{Router, RouterConfig, RouterStats, TransportOutput};
pub use server::{ListenerConfig, ProtocolServer, ServerContext};
pub use session::{Session, SessionMode, SessionRegistry, SessionState, SessionTrack, Transport, TransportType};
pub use sink::{MediaPacket, QueueFullBehavior, Sink, SinkConfig, SinkId, SinkReceiver, SinkStats, SinkStatsSnapshot};
pub use source::{Source, SourceId, SourceStats, SourceStatsSnapshot};
pub use stream::{Stream, StreamId, StreamRegistry, StreamState, StreamStats, TrackId, TrackInfo, TrackType};
