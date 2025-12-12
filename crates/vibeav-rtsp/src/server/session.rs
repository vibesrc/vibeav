//! RTSP session management.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use tokio::sync::RwLock;

use crate::transport::Transport;

/// Session state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Session created, waiting for SETUP.
    Init,
    /// SETUP complete, ready for PLAY/RECORD.
    Ready,
    /// Actively playing (sending media to client).
    Playing,
    /// Actively recording (receiving media from client).
    Recording,
    /// Session ended.
    Ended,
}

/// Session mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionMode {
    /// Client is receiving media (DESCRIBE → SETUP → PLAY).
    Play,
    /// Client is publishing media (ANNOUNCE → SETUP → RECORD).
    Record,
}

/// Track setup information.
#[derive(Debug, Clone)]
pub struct TrackSetup {
    /// Track control URL (e.g., "trackID=0").
    pub control: String,
    /// Stream ID in the router.
    pub stream_id: String,
    /// Transport configuration.
    pub transport: Transport,
    /// Interleaved channel for RTP (if TCP).
    pub rtp_channel: Option<u8>,
    /// Interleaved channel for RTCP (if TCP).
    pub rtcp_channel: Option<u8>,
}

/// Counter for generating unique session IDs.
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generate a unique session ID.
fn generate_session_id() -> String {
    let counter = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{:08X}{:08X}", timestamp as u32, counter as u32)
}

/// An RTSP session.
#[derive(Debug)]
pub struct RtspSession {
    /// Unique session ID.
    id: String,
    /// Current state.
    state: RwLock<SessionState>,
    /// Session mode (play or record).
    mode: RwLock<SessionMode>,
    /// Stream path (e.g., "/live/camera1").
    stream_path: RwLock<Option<String>>,
    /// Track setups (control URL -> setup).
    tracks: RwLock<HashMap<String, TrackSetup>>,
    /// Core session ID (for router integration).
    core_session_id: RwLock<Option<String>>,
    /// Last activity time.
    last_activity: RwLock<Instant>,
    /// Creation time.
    created_at: Instant,
}

impl RtspSession {
    /// Create a new session.
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            id: generate_session_id(),
            state: RwLock::new(SessionState::Init),
            mode: RwLock::new(SessionMode::Play),
            stream_path: RwLock::new(None),
            tracks: RwLock::new(HashMap::new()),
            core_session_id: RwLock::new(None),
            last_activity: RwLock::new(now),
            created_at: now,
        }
    }

    /// Get session ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get current state.
    pub async fn state(&self) -> SessionState {
        *self.state.read().await
    }

    /// Set state.
    pub async fn set_state(&self, state: SessionState) {
        *self.state.write().await = state;
    }

    /// Get mode.
    pub async fn mode(&self) -> SessionMode {
        *self.mode.read().await
    }

    /// Set mode.
    pub async fn set_mode(&self, mode: SessionMode) {
        *self.mode.write().await = mode;
    }

    /// Get stream path.
    pub async fn stream_path(&self) -> Option<String> {
        self.stream_path.read().await.clone()
    }

    /// Set stream path.
    pub async fn set_stream_path(&self, path: String) {
        *self.stream_path.write().await = Some(path);
    }

    /// Add a track setup.
    pub async fn add_track(&self, control: String, setup: TrackSetup) {
        self.tracks.write().await.insert(control, setup);
    }

    /// Get a track setup.
    pub async fn get_track(&self, control: &str) -> Option<TrackSetup> {
        self.tracks.read().await.get(control).cloned()
    }

    /// Get all tracks.
    pub async fn tracks(&self) -> Vec<TrackSetup> {
        self.tracks.read().await.values().cloned().collect()
    }

    /// Get track by interleaved channel.
    pub async fn track_by_channel(&self, channel: u8) -> Option<TrackSetup> {
        let tracks = self.tracks.read().await;
        tracks
            .values()
            .find(|t| t.rtp_channel == Some(channel) || t.rtcp_channel == Some(channel))
            .cloned()
    }

    /// Set core session ID.
    pub async fn set_core_session_id(&self, id: String) {
        *self.core_session_id.write().await = Some(id);
    }

    /// Get core session ID.
    pub async fn core_session_id(&self) -> Option<String> {
        self.core_session_id.read().await.clone()
    }

    /// Touch session (update last activity).
    pub async fn touch(&self) {
        *self.last_activity.write().await = Instant::now();
    }

    /// Check if session is timed out.
    pub async fn is_timeout(&self, timeout_secs: u64) -> bool {
        self.last_activity.read().await.elapsed().as_secs() > timeout_secs
    }

    /// Get session age.
    pub fn age(&self) -> std::time::Duration {
        self.created_at.elapsed()
    }
}

impl Default for RtspSession {
    fn default() -> Self {
        Self::new()
    }
}
