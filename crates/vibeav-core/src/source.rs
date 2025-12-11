//! Source management.
//!
//! A source publishes media to a stream.
//! Sources are the input side of the router - media flows from them.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Unique source identifier.
pub type SourceId = String;

/// Source statistics.
#[derive(Debug, Default)]
pub struct SourceStats {
    /// Packets received from this source.
    pub packets_received: AtomicU64,
    /// Bytes received from this source.
    pub bytes_received: AtomicU64,
}

impl SourceStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_received(&self, bytes: usize) {
        self.packets_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> SourceStatsSnapshot {
        SourceStatsSnapshot {
            packets_received: self.packets_received.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of source statistics.
#[derive(Debug, Clone, Default)]
pub struct SourceStatsSnapshot {
    pub packets_received: u64,
    pub bytes_received: u64,
}

/// A source publishing media to a stream.
#[derive(Debug)]
pub struct Source {
    /// Source ID.
    pub id: SourceId,
    /// Stream this source publishes to.
    pub stream_id: String,
    /// When this source was created.
    pub created_at: Instant,
    /// Statistics.
    stats: SourceStats,
}

impl Source {
    /// Create a new source.
    pub fn new(id: impl Into<SourceId>, stream_id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            stream_id: stream_id.into(),
            created_at: Instant::now(),
            stats: SourceStats::new(),
        }
    }

    /// Record a packet received from this source.
    pub fn record_received(&self, bytes: usize) {
        self.stats.record_received(bytes);
    }

    /// Get source statistics.
    pub fn stats(&self) -> &SourceStats {
        &self.stats
    }
}
