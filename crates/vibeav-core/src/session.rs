//! Session management.
//!
//! A session represents a client connection with its sinks.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::RwLock;

use crate::error::{CoreError, Result};
use crate::sink::SinkId;

/// Unique session identifier.
pub type SessionId = String;

/// Session mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionMode {
    /// Playback (server sends media to client).
    Play,
    /// Record (client sends media to server).
    Record,
}

/// Session state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Session is created but not started.
    Init,
    /// Session is ready (SETUP complete).
    Ready,
    /// Session is playing (receiving media from server).
    Playing,
    /// Session is recording (sending media to server).
    Recording,
    /// Session is paused.
    Paused,
    /// Session has ended.
    Ended,
}

/// Transport information.
#[derive(Debug, Clone)]
pub struct Transport {
    /// Transport type.
    pub transport_type: TransportType,
    /// Interleaved channels (for TCP).
    pub interleaved: Option<(u8, u8)>,
    /// Client ports (for UDP).
    pub client_ports: Option<(u16, u16)>,
    /// Server ports (for UDP).
    pub server_ports: Option<(u16, u16)>,
}

/// Transport type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportType {
    /// TCP interleaved.
    TcpInterleaved,
    /// UDP unicast.
    UdpUnicast,
    /// UDP multicast.
    UdpMulticast,
}

/// Track setup within a session.
#[derive(Debug, Clone)]
pub struct SessionTrack {
    /// Track control URL.
    pub control_url: String,
    /// Transport for this track.
    pub transport: Transport,
    /// Associated stream ID.
    pub stream_id: String,
}

/// A client session.
#[derive(Debug)]
pub struct Session {
    /// Session ID.
    pub id: SessionId,
    /// Current state.
    state: RwLock<SessionState>,
    /// Session mode.
    mode: RwLock<SessionMode>,
    /// Tracks in this session.
    tracks: RwLock<HashMap<String, SessionTrack>>,
    /// Sink IDs owned by this session.
    sinks: RwLock<Vec<SinkId>>,
    /// Creation time.
    pub created_at: Instant,
    /// Last activity time.
    last_activity: RwLock<Instant>,
}

impl Session {
    /// Create a new session.
    pub fn new(id: impl Into<SessionId>) -> Self {
        let now = Instant::now();
        Self {
            id: id.into(),
            state: RwLock::new(SessionState::Init),
            mode: RwLock::new(SessionMode::Play),
            tracks: RwLock::new(HashMap::new()),
            sinks: RwLock::new(Vec::new()),
            created_at: now,
            last_activity: RwLock::new(now),
        }
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

    /// Add a track.
    pub async fn add_track(&self, control_url: String, track: SessionTrack) {
        self.tracks.write().await.insert(control_url, track);
    }

    /// Get track by control URL.
    pub async fn get_track(&self, control_url: &str) -> Option<SessionTrack> {
        self.tracks.read().await.get(control_url).cloned()
    }

    /// Get all tracks.
    pub async fn tracks(&self) -> Vec<SessionTrack> {
        self.tracks.read().await.values().cloned().collect()
    }

    /// Track count.
    pub async fn track_count(&self) -> usize {
        self.tracks.read().await.len()
    }

    /// Add sink ID.
    pub async fn add_sink(&self, sink_id: SinkId) {
        self.sinks.write().await.push(sink_id);
    }

    /// Get sink IDs.
    pub async fn sinks(&self) -> Vec<SinkId> {
        self.sinks.read().await.clone()
    }

    /// Record activity.
    pub async fn touch(&self) {
        *self.last_activity.write().await = Instant::now();
    }

    /// Get last activity time.
    pub async fn last_activity(&self) -> Instant {
        *self.last_activity.read().await
    }

    /// Check if session is timed out.
    pub async fn is_timeout(&self, timeout_secs: u64) -> bool {
        self.last_activity().await.elapsed().as_secs() > timeout_secs
    }
}

/// Session registry.
#[derive(Debug, Default)]
pub struct SessionRegistry {
    sessions: RwLock<HashMap<SessionId, Arc<Session>>>,
}

impl SessionRegistry {
    /// Create a new registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new session.
    pub async fn create(&self, session_id: impl Into<SessionId>) -> Result<Arc<Session>> {
        let session_id = session_id.into();
        let mut sessions = self.sessions.write().await;
        if sessions.contains_key(&session_id) {
            return Err(CoreError::SessionExists(session_id));
        }
        let session = Arc::new(Session::new(session_id.clone()));
        sessions.insert(session_id, session.clone());
        Ok(session)
    }

    /// Get session by ID.
    pub async fn get(&self, session_id: &str) -> Option<Arc<Session>> {
        self.sessions.read().await.get(session_id).cloned()
    }

    /// Remove session.
    pub async fn remove(&self, session_id: &str) -> Option<Arc<Session>> {
        self.sessions.write().await.remove(session_id)
    }

    /// List all session IDs.
    pub async fn list(&self) -> Vec<SessionId> {
        self.sessions.read().await.keys().cloned().collect()
    }

    /// Get total number of sessions.
    pub async fn count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Remove timed-out sessions.
    pub async fn cleanup_timeout(&self, timeout_secs: u64) -> Vec<Arc<Session>> {
        let mut sessions = self.sessions.write().await;
        let mut removed = Vec::new();

        let timed_out: Vec<SessionId> = {
            let mut ids = Vec::new();
            for (id, session) in sessions.iter() {
                if session.is_timeout(timeout_secs).await {
                    ids.push(id.clone());
                }
            }
            ids
        };

        for id in timed_out {
            if let Some(session) = sessions.remove(&id) {
                removed.push(session);
            }
        }

        removed
    }
}
