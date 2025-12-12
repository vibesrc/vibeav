//! Main router engine.
//!
//! Routes RTP packets from sources to sinks with zero-copy forwarding.

use std::collections::HashMap;
use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::RwLock;
use tracing::{debug, trace, warn};

use crate::attachment::{Attachment, AttachmentId};
use crate::error::{CoreError, Result};
use crate::session::{Session, SessionRegistry};
use crate::sink::{MediaPacket, Sink, SinkConfig, SinkId, SinkReceiver};
use crate::source::{Source, SourceId};
use crate::stream::{Stream, StreamId, StreamRegistry, StreamState, TrackId};

/// Router configuration.
#[derive(Debug, Clone)]
pub struct RouterConfig {
    /// Default sink queue size.
    pub default_queue_size: usize,
    /// Session timeout in seconds.
    pub session_timeout_secs: u64,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            default_queue_size: 256,
            session_timeout_secs: 60,
        }
    }
}

/// Trait for receiving packets from the router.
pub trait TransportOutput: Send + Sync {
    /// Send RTP packet.
    fn send_rtp(&self, data: &[u8]) -> Result<()>;

    /// Send RTCP packet.
    fn send_rtcp(&self, data: &[u8]) -> Result<()>;
}

/// Main router engine.
#[derive(Debug)]
pub struct Router {
    /// Configuration.
    config: RouterConfig,
    /// Stream registry.
    streams: StreamRegistry,
    /// Session registry.
    sessions: SessionRegistry,
    /// Source registry (source_id -> Source).
    sources: RwLock<HashMap<SourceId, Source>>,
    /// Sink registry (sink_id -> Sink).
    sinks: RwLock<HashMap<SinkId, Sink>>,
    /// Stream -> Sink mappings (for stream-level subscriptions).
    stream_sinks: RwLock<HashMap<StreamId, Vec<SinkId>>>,
    /// Attachment registry (attachment_id -> Attachment).
    attachments: RwLock<HashMap<AttachmentId, Attachment>>,
    /// Payload type -> Attachments (for track-level routing).
    /// Key is (stream_id, payload_type).
    payload_attachments: RwLock<HashMap<(StreamId, u8), Vec<AttachmentId>>>,
}

impl Router {
    /// Create a new router with default configuration.
    pub fn new() -> Self {
        Self::with_config(RouterConfig::default())
    }

    /// Create a new router with custom configuration.
    pub fn with_config(config: RouterConfig) -> Self {
        Self {
            config,
            streams: StreamRegistry::new(),
            sessions: SessionRegistry::new(),
            sources: RwLock::new(HashMap::new()),
            sinks: RwLock::new(HashMap::new()),
            stream_sinks: RwLock::new(HashMap::new()),
            attachments: RwLock::new(HashMap::new()),
            payload_attachments: RwLock::new(HashMap::new()),
        }
    }

    // =========================================================================
    // Stream Management
    // =========================================================================

    /// Register a new stream.
    pub async fn create_stream(&self, stream_id: impl Into<StreamId>) -> Result<Arc<Stream>> {
        let stream = self.streams.register(stream_id).await?;
        debug!(stream_id = %stream.id, "Stream created");
        Ok(stream)
    }

    /// Get a stream by ID.
    pub async fn get_stream(&self, stream_id: &str) -> Option<Arc<Stream>> {
        self.streams.get(stream_id).await
    }

    /// Remove a stream.
    pub async fn remove_stream(&self, stream_id: &str) -> Result<()> {
        // Remove all sinks attached to this stream
        {
            let mut stream_sinks = self.stream_sinks.write().await;
            if let Some(sink_ids) = stream_sinks.remove(stream_id) {
                let mut sinks = self.sinks.write().await;
                for id in sink_ids {
                    sinks.remove(&id);
                }
            }
        }

        // Remove all attachments for this stream
        {
            let mut attachments = self.attachments.write().await;
            let mut payload_attachments = self.payload_attachments.write().await;

            // Remove from attachments registry
            attachments.retain(|_, a| a.track_id.stream_id != stream_id);

            // Remove from payload lookup
            payload_attachments.retain(|(sid, _), _| sid != stream_id);
        }

        // Remove source if any
        {
            let mut sources = self.sources.write().await;
            sources.retain(|_, source| source.stream_id != stream_id);
        }

        // Remove the stream
        self.streams.remove(stream_id).await;
        debug!(stream_id = %stream_id, "Stream removed");
        Ok(())
    }

    /// List all stream IDs.
    pub async fn list_streams(&self) -> Vec<StreamId> {
        self.streams.list().await
    }

    // =========================================================================
    // Session Management
    // =========================================================================

    /// Create a new session.
    pub async fn create_session(&self, session_id: impl Into<String>) -> Result<Arc<Session>> {
        let session = self.sessions.create(session_id).await?;
        debug!(session_id = %session.id, "Session created");
        Ok(session)
    }

    /// Get a session by ID.
    pub async fn get_session(&self, session_id: &str) -> Option<Arc<Session>> {
        self.sessions.get(session_id).await
    }

    /// Remove a session and its sinks.
    pub async fn remove_session(&self, session_id: &str) -> Result<()> {
        if let Some(session) = self.sessions.remove(session_id).await {
            // Remove all sinks owned by this session
            let sink_ids = session.sinks().await;
            for sink_id in sink_ids {
                self.remove_sink(&sink_id).await?;
            }
            debug!(session_id = %session_id, "Session removed");
        }
        Ok(())
    }

    // =========================================================================
    // Sink Management
    // =========================================================================

    /// Add a sink to receive from a stream.
    ///
    /// Returns a receiver for media packets.
    pub async fn add_sink(
        &self,
        stream_id: &str,
        sink_id: impl Into<SinkId>,
        config: Option<SinkConfig>,
    ) -> Result<SinkReceiver> {
        let sink_id = sink_id.into();
        let config = config.unwrap_or_default();

        // Check stream exists
        let stream = self
            .streams
            .get(stream_id)
            .await
            .ok_or_else(|| CoreError::StreamNotFound(stream_id.to_string()))?;

        // Create sink
        let (sink, receiver) = Sink::new(sink_id.clone(), stream_id, config);

        // Register sink
        {
            let mut sinks = self.sinks.write().await;
            sinks.insert(sink_id.clone(), sink);
        }

        // Add to stream's sink list
        {
            let mut stream_sinks = self.stream_sinks.write().await;
            stream_sinks
                .entry(stream_id.to_string())
                .or_default()
                .push(sink_id.clone());
        }

        // Add to stream
        stream.add_sink(sink_id.clone()).await;

        debug!(stream_id = %stream_id, sink_id = %sink_id, "Sink added");
        Ok(receiver)
    }

    /// Create a standalone sink (not attached to any stream).
    ///
    /// Use this for track-level subscriptions where you want to receive
    /// only specific payload types via `attach_track()`.
    pub async fn create_sink(
        &self,
        sink_id: impl Into<SinkId>,
        stream_id: &str,
        config: Option<SinkConfig>,
    ) -> Result<SinkReceiver> {
        let sink_id = sink_id.into();
        let config = config.unwrap_or_default();

        // Check stream exists
        self.streams
            .get(stream_id)
            .await
            .ok_or_else(|| CoreError::StreamNotFound(stream_id.to_string()))?;

        // Create sink
        let (sink, receiver) = Sink::new(sink_id.clone(), stream_id, config);

        // Register sink (but don't add to stream_sinks - that's for stream-level subscriptions)
        {
            let mut sinks = self.sinks.write().await;
            sinks.insert(sink_id.clone(), sink);
        }

        debug!(stream_id = %stream_id, sink_id = %sink_id, "Standalone sink created");
        Ok(receiver)
    }

    /// Remove a sink from a stream.
    pub async fn remove_sink_from_stream(&self, stream_id: &str, sink_id: &str) -> Result<()> {
        self.remove_sink(sink_id).await?;

        // Remove from stream
        if let Some(stream) = self.streams.get(stream_id).await {
            stream.remove_sink(&sink_id.to_string()).await;
        }

        Ok(())
    }

    /// Remove a standalone sink.
    pub async fn destroy_sink(&self, sink_id: &str) -> Result<()> {
        self.remove_sink(sink_id).await
    }

    /// Remove a sink.
    async fn remove_sink(&self, sink_id: &str) -> Result<()> {
        // Remove from sink registry
        let sink = {
            let mut sinks = self.sinks.write().await;
            sinks.remove(sink_id)
        };

        if let Some(sink) = sink {
            // Remove from stream_sinks
            {
                let mut stream_sinks = self.stream_sinks.write().await;
                if let Some(sinks) = stream_sinks.get_mut(&sink.stream_id) {
                    sinks.retain(|id| id != sink_id);
                }
            }

            // Remove any attachments referencing this sink
            {
                let mut attachments = self.attachments.write().await;
                let mut payload_attachments = self.payload_attachments.write().await;

                // Find and collect attachment IDs to remove
                let attachment_ids_to_remove: Vec<AttachmentId> = attachments
                    .iter()
                    .filter(|(_, a)| a.sink_id == sink_id)
                    .map(|(id, _)| id.clone())
                    .collect();

                // Remove from attachments registry
                for id in &attachment_ids_to_remove {
                    attachments.remove(id);
                }

                // Remove from payload lookup
                for lists in payload_attachments.values_mut() {
                    lists.retain(|id| !attachment_ids_to_remove.contains(id));
                }
            }

            debug!(sink_id = %sink_id, "Sink removed");
        }

        Ok(())
    }

    // =========================================================================
    // Attachment Management (Track-Level Subscriptions)
    // =========================================================================

    /// Attach a sink to a specific track (by payload type).
    ///
    /// This enables track-level subscriptions where a sink only receives
    /// packets matching a specific payload type (e.g., video only, audio only).
    pub async fn attach_track(
        &self,
        stream_id: &str,
        sink_id: &str,
        payload_type: u8,
        attachment_id: impl Into<AttachmentId>,
    ) -> Result<()> {
        let attachment_id = attachment_id.into();

        // Check stream exists
        self.streams
            .get(stream_id)
            .await
            .ok_or_else(|| CoreError::StreamNotFound(stream_id.to_string()))?;

        // Check sink exists
        {
            let sinks = self.sinks.read().await;
            if !sinks.contains_key(sink_id) {
                return Err(CoreError::SinkNotFound(sink_id.to_string()));
            }
        }

        // Create track ID for the attachment
        // Note: We use payload_type as a simplified track identifier
        let track_id = TrackId::new(stream_id, crate::stream::TrackType::Video, payload_type);

        // Create and register attachment
        let attachment = Attachment::new(attachment_id.clone(), track_id, sink_id);
        {
            let mut attachments = self.attachments.write().await;
            attachments.insert(attachment_id.clone(), attachment);
        }

        // Add to payload type lookup
        {
            let mut payload_attachments = self.payload_attachments.write().await;
            payload_attachments
                .entry((stream_id.to_string(), payload_type))
                .or_default()
                .push(attachment_id.clone());
        }

        debug!(
            stream_id = %stream_id,
            sink_id = %sink_id,
            payload_type = payload_type,
            attachment_id = %attachment_id,
            "Track attachment created"
        );
        Ok(())
    }

    /// Remove a track attachment.
    pub async fn detach_track(&self, attachment_id: &str) -> Result<()> {
        // Remove attachment
        let attachment = {
            let mut attachments = self.attachments.write().await;
            attachments.remove(attachment_id)
        };

        if let Some(attachment) = attachment {
            // Remove from payload type lookup
            let stream_id = &attachment.track_id.stream_id;
            let payload_type = attachment.track_id.index;

            let mut payload_attachments = self.payload_attachments.write().await;
            if let Some(attachments) = payload_attachments.get_mut(&(stream_id.clone(), payload_type)) {
                attachments.retain(|id| id != attachment_id);
            }

            debug!(attachment_id = %attachment_id, "Track attachment removed");
        }

        Ok(())
    }

    /// Get an attachment by ID.
    pub async fn get_attachment(&self, attachment_id: &str) -> Option<Attachment> {
        self.attachments.read().await.get(attachment_id).cloned()
    }

    /// List all attachments for a stream.
    pub async fn list_attachments(&self, stream_id: &str) -> Vec<Attachment> {
        let attachments = self.attachments.read().await;
        attachments
            .values()
            .filter(|a| a.track_id.stream_id == stream_id)
            .cloned()
            .collect()
    }

    // =========================================================================
    // Source Management
    // =========================================================================

    /// Set a source for a stream.
    pub async fn set_source(&self, stream_id: &str, source_id: &str) -> Result<()> {
        let stream = self
            .streams
            .get(stream_id)
            .await
            .ok_or_else(|| CoreError::StreamNotFound(stream_id.to_string()))?;

        stream.set_source(source_id.to_string()).await?;
        stream.set_state(StreamState::Active).await;

        // Create and register source
        let source = Source::new(source_id, stream_id);
        self.sources.write().await.insert(source_id.to_string(), source);

        debug!(stream_id = %stream_id, source_id = %source_id, "Source set");
        Ok(())
    }

    /// Clear source for a stream.
    pub async fn clear_source(&self, stream_id: &str) -> Result<()> {
        if let Some(stream) = self.streams.get(stream_id).await {
            if let Some(source_id) = stream.source().await {
                self.sources.write().await.remove(&source_id);
            }
            stream.clear_source().await;
            stream.set_state(StreamState::Idle).await;
            debug!(stream_id = %stream_id, "Source cleared");
        }
        Ok(())
    }

    // =========================================================================
    // Packet Forwarding
    // =========================================================================

    /// Receive an RTP packet from a source (zero-copy).
    ///
    /// This is the main ingress point for media.
    /// Packets are routed to:
    /// 1. Stream-level sinks (receive all packets for the stream)
    /// 2. Track-level attachments (receive only packets matching their payload type)
    pub async fn on_rtp(&self, stream_id: &str, data: Bytes) -> Result<()> {
        debug!(stream_id = %stream_id, len = data.len(), "on_rtp called");

        let stream = self
            .streams
            .get(stream_id)
            .await
            .ok_or_else(|| CoreError::StreamNotFound(stream_id.to_string()))?;

        // Record statistics
        stream.stats().record_received(data.len());

        // Create media packet (zero-copy - Bytes is Arc'd internally)
        let packet = MediaPacket::from_rtp(data)
            .ok_or_else(|| CoreError::SendFailed("invalid RTP packet".into()))?;

        let payload_type = packet.payload_type;

        // Collect all sink IDs to forward to:
        // 1. Stream-level sinks (get all packets)
        // 2. Track-level attachment sinks (filtered by payload type)
        let mut target_sink_ids: Vec<SinkId> = Vec::new();

        // Add stream-level sinks
        {
            let stream_sinks = self.stream_sinks.read().await;
            if let Some(sinks) = stream_sinks.get(stream_id) {
                target_sink_ids.extend(sinks.iter().cloned());
            }
        }

        // Add track-level attachment sinks (filtered by payload type)
        {
            let payload_attachments = self.payload_attachments.read().await;
            let attachments = self.attachments.read().await;

            if let Some(attachment_ids) = payload_attachments.get(&(stream_id.to_string(), payload_type)) {
                for attachment_id in attachment_ids {
                    if let Some(attachment) = attachments.get(attachment_id) {
                        if attachment.active {
                            // Avoid duplicates (sink might be both stream-level and attachment)
                            if !target_sink_ids.contains(&attachment.sink_id) {
                                target_sink_ids.push(attachment.sink_id.clone());
                            }
                        }
                    }
                }
            }
        }

        if target_sink_ids.is_empty() {
            debug!(stream_id = %stream_id, "No sinks, dropping packet");
            return Ok(());
        }

        debug!(stream_id = %stream_id, sink_count = target_sink_ids.len(), "Forwarding to sinks");

        // Forward to all target sinks
        let mut forwarded = 0;
        let mut dropped = 0;

        {
            let sinks = self.sinks.read().await;
            for sink_id in &target_sink_ids {
                if let Some(sink) = sinks.get(sink_id) {
                    match sink.try_send(packet.clone()) {
                        Ok(()) => forwarded += 1,
                        Err(CoreError::QueueFull) => {
                            dropped += 1;
                            trace!(sink_id = %sink_id, "Queue full, dropping packet");
                        }
                        Err(CoreError::ChannelClosed) => {
                            debug!(sink_id = %sink_id, "Channel closed");
                        }
                        Err(e) => {
                            warn!(sink_id = %sink_id, error = %e, "Send failed");
                        }
                    }
                }
            }
        }

        stream.stats().record_forwarded(forwarded);
        stream.stats().record_dropped(dropped);

        trace!(
            stream_id = %stream_id,
            payload_type = payload_type,
            forwarded = forwarded,
            dropped = dropped,
            "Packet forwarded"
        );

        Ok(())
    }

    /// Receive an RTCP packet from a source.
    pub async fn on_rtcp(&self, stream_id: &str, _data: Bytes) -> Result<()> {
        // For now, just log RTCP reception
        trace!(stream_id = %stream_id, "RTCP received");
        Ok(())
    }

    // =========================================================================
    // Maintenance
    // =========================================================================

    /// Clean up timed-out sessions.
    pub async fn cleanup_sessions(&self) -> Vec<Arc<Session>> {
        let removed = self.sessions.cleanup_timeout(self.config.session_timeout_secs).await;

        for session in &removed {
            // Remove sinks owned by this session
            let sink_ids = session.sinks().await;
            for sink_id in sink_ids {
                let _ = self.remove_sink(&sink_id).await;
            }
        }

        removed
    }

    /// Get router statistics.
    pub async fn stats(&self) -> RouterStats {
        RouterStats {
            stream_count: self.streams.count().await,
            session_count: self.sessions.count().await,
            source_count: self.sources.read().await.len(),
            sink_count: self.sinks.read().await.len(),
        }
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

/// Router statistics.
#[derive(Debug, Clone, Default)]
pub struct RouterStats {
    pub stream_count: usize,
    pub session_count: usize,
    pub source_count: usize,
    pub sink_count: usize,
}
