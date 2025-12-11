//! Attachment management.
//!
//! An attachment binds a track to a sink, enabling track-level subscriptions.
//! This allows clients to receive only video, only audio, or specific tracks.

use crate::sink::SinkId;
use crate::stream::TrackId;

/// Unique attachment identifier.
pub type AttachmentId = String;

/// An attachment binding a track to a sink.
#[derive(Debug, Clone)]
pub struct Attachment {
    /// Attachment ID.
    pub id: AttachmentId,
    /// The track being attached.
    pub track_id: TrackId,
    /// The sink receiving packets.
    pub sink_id: SinkId,
    /// Whether this attachment is active.
    pub active: bool,
}

impl Attachment {
    /// Create a new attachment.
    pub fn new(
        id: impl Into<AttachmentId>,
        track_id: TrackId,
        sink_id: impl Into<SinkId>,
    ) -> Self {
        Self {
            id: id.into(),
            track_id,
            sink_id: sink_id.into(),
            active: true,
        }
    }

    /// Activate this attachment.
    pub fn activate(&mut self) {
        self.active = true;
    }

    /// Deactivate this attachment.
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}
