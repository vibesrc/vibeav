//! Stream state management.
//!
//! A stream represents a media source (e.g., camera feed, recorded file).
//! Streams can have one source and multiple sinks.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::error::{CoreError, Result};
use crate::sink::SinkId;
use crate::source::SourceId;

/// Unique stream identifier.
pub type StreamId = String;

/// Track type within a stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrackType {
    Video,
    Audio,
    Data,
}

/// Track identifier within a stream.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TrackId {
    pub stream_id: StreamId,
    pub track_type: TrackType,
    pub index: u8,
}

impl TrackId {
    pub fn new(stream_id: impl Into<StreamId>, track_type: TrackType, index: u8) -> Self {
        Self {
            stream_id: stream_id.into(),
            track_type,
            index,
        }
    }

    pub fn video(stream_id: impl Into<StreamId>) -> Self {
        Self::new(stream_id, TrackType::Video, 0)
    }

    pub fn audio(stream_id: impl Into<StreamId>) -> Self {
        Self::new(stream_id, TrackType::Audio, 0)
    }
}

/// Track metadata.
#[derive(Debug, Clone)]
pub struct TrackInfo {
    /// Track type.
    pub track_type: TrackType,
    /// Payload type (from SDP).
    pub payload_type: u8,
    /// Clock rate in Hz.
    pub clock_rate: u32,
    /// Encoding name (e.g., "H264", "AAC").
    pub encoding: String,
    /// Format-specific parameters (from fmtp).
    pub parameters: Option<String>,
}

/// Stream statistics.
#[derive(Debug, Default)]
pub struct StreamStats {
    /// Total RTP packets received.
    pub packets_received: AtomicU64,
    /// Total bytes received.
    pub bytes_received: AtomicU64,
    /// Total packets forwarded to sinks.
    pub packets_forwarded: AtomicU64,
    /// Packets dropped (queue full, etc.).
    pub packets_dropped: AtomicU64,
}

impl StreamStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_received(&self, bytes: usize) {
        self.packets_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    pub fn record_forwarded(&self, count: usize) {
        self.packets_forwarded.fetch_add(count as u64, Ordering::Relaxed);
    }

    pub fn record_dropped(&self, count: usize) {
        self.packets_dropped.fetch_add(count as u64, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> StreamStatsSnapshot {
        StreamStatsSnapshot {
            packets_received: self.packets_received.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            packets_forwarded: self.packets_forwarded.load(Ordering::Relaxed),
            packets_dropped: self.packets_dropped.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of stream statistics.
#[derive(Debug, Clone, Default)]
pub struct StreamStatsSnapshot {
    pub packets_received: u64,
    pub bytes_received: u64,
    pub packets_forwarded: u64,
    pub packets_dropped: u64,
}

/// Stream state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    /// Stream is registered but not active.
    Idle,
    /// Stream is receiving media.
    Active,
    /// Stream is paused.
    Paused,
    /// Stream has ended.
    Ended,
}

/// A media stream.
#[derive(Debug)]
pub struct Stream {
    /// Stream ID.
    pub id: StreamId,
    /// Current state.
    state: RwLock<StreamState>,
    /// Track information.
    tracks: RwLock<Vec<TrackInfo>>,
    /// Active sinks.
    sinks: RwLock<Vec<SinkId>>,
    /// Source ID (if any).
    source: RwLock<Option<SourceId>>,
    /// Statistics.
    stats: StreamStats,
}

impl Stream {
    /// Create a new stream.
    pub fn new(id: impl Into<StreamId>) -> Self {
        Self {
            id: id.into(),
            state: RwLock::new(StreamState::Idle),
            tracks: RwLock::new(Vec::new()),
            sinks: RwLock::new(Vec::new()),
            source: RwLock::new(None),
            stats: StreamStats::new(),
        }
    }

    /// Get stream state.
    pub async fn state(&self) -> StreamState {
        *self.state.read().await
    }

    /// Set stream state.
    pub async fn set_state(&self, state: StreamState) {
        *self.state.write().await = state;
    }

    /// Add a track to the stream.
    pub async fn add_track(&self, track: TrackInfo) {
        self.tracks.write().await.push(track);
    }

    /// Get track information.
    pub async fn tracks(&self) -> Vec<TrackInfo> {
        self.tracks.read().await.clone()
    }

    /// Set source.
    pub async fn set_source(&self, source_id: SourceId) -> Result<()> {
        let mut source = self.source.write().await;
        if source.is_some() {
            return Err(CoreError::SourceExists(self.id.clone()));
        }
        *source = Some(source_id);
        Ok(())
    }

    /// Clear source.
    pub async fn clear_source(&self) {
        *self.source.write().await = None;
    }

    /// Check if stream has a source.
    pub async fn has_source(&self) -> bool {
        self.source.read().await.is_some()
    }

    /// Get source ID.
    pub async fn source(&self) -> Option<SourceId> {
        self.source.read().await.clone()
    }

    /// Add a sink.
    pub async fn add_sink(&self, sink_id: SinkId) {
        self.sinks.write().await.push(sink_id);
    }

    /// Remove a sink.
    pub async fn remove_sink(&self, sink_id: &SinkId) {
        self.sinks.write().await.retain(|id| id != sink_id);
    }

    /// Get sink IDs.
    pub async fn sink_ids(&self) -> Vec<SinkId> {
        self.sinks.read().await.clone()
    }

    /// Get sink count.
    pub async fn sink_count(&self) -> usize {
        self.sinks.read().await.len()
    }

    /// Get statistics.
    pub fn stats(&self) -> &StreamStats {
        &self.stats
    }
}

/// Stream registry.
#[derive(Debug, Default)]
pub struct StreamRegistry {
    streams: RwLock<HashMap<StreamId, Arc<Stream>>>,
}

impl StreamRegistry {
    /// Create a new registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new stream.
    pub async fn register(&self, stream_id: impl Into<StreamId>) -> Result<Arc<Stream>> {
        let stream_id = stream_id.into();
        let mut streams = self.streams.write().await;
        if streams.contains_key(&stream_id) {
            return Err(CoreError::StreamExists(stream_id));
        }
        let stream = Arc::new(Stream::new(stream_id.clone()));
        streams.insert(stream_id, stream.clone());
        Ok(stream)
    }

    /// Get a stream by ID.
    pub async fn get(&self, stream_id: &str) -> Option<Arc<Stream>> {
        self.streams.read().await.get(stream_id).cloned()
    }

    /// Remove a stream.
    pub async fn remove(&self, stream_id: &str) -> Option<Arc<Stream>> {
        self.streams.write().await.remove(stream_id)
    }

    /// List all stream IDs.
    pub async fn list(&self) -> Vec<StreamId> {
        self.streams.read().await.keys().cloned().collect()
    }

    /// Get total number of streams.
    pub async fn count(&self) -> usize {
        self.streams.read().await.len()
    }
}
