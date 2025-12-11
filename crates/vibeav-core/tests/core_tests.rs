//! Core router tests.

use bytes::Bytes;
use vibeav_core::{
    CoreError, MediaPacket, QueueFullBehavior, Router, SessionMode,
    SessionState, StreamState, SinkConfig, TrackId, TrackInfo, TrackType,
};

// =============================================================================
// Stream Tests
// =============================================================================

#[tokio::test]
async fn test_create_stream() {
    let router = Router::new();
    let stream = router.create_stream("test-stream").await.unwrap();
    assert_eq!(stream.id, "test-stream");
}

#[tokio::test]
async fn test_create_duplicate_stream() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();

    let result = router.create_stream("stream1").await;
    assert!(matches!(result, Err(CoreError::StreamExists(_))));
}

#[tokio::test]
async fn test_get_stream() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();

    let stream = router.get_stream("stream1").await;
    assert!(stream.is_some());
    assert_eq!(stream.unwrap().id, "stream1");

    let missing = router.get_stream("nonexistent").await;
    assert!(missing.is_none());
}

#[tokio::test]
async fn test_remove_stream() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();

    router.remove_stream("stream1").await.unwrap();
    assert!(router.get_stream("stream1").await.is_none());
}

#[tokio::test]
async fn test_list_streams() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();
    router.create_stream("stream2").await.unwrap();
    router.create_stream("stream3").await.unwrap();

    let mut streams = router.list_streams().await;
    streams.sort();
    assert_eq!(streams, vec!["stream1", "stream2", "stream3"]);
}

#[tokio::test]
async fn test_stream_state() {
    let router = Router::new();
    let stream = router.create_stream("stream1").await.unwrap();

    assert_eq!(stream.state().await, StreamState::Idle);
    stream.set_state(StreamState::Active).await;
    assert_eq!(stream.state().await, StreamState::Active);
}

#[tokio::test]
async fn test_stream_tracks() {
    let router = Router::new();
    let stream = router.create_stream("stream1").await.unwrap();

    stream
        .add_track(TrackInfo {
            track_type: TrackType::Video,
            payload_type: 96,
            clock_rate: 90000,
            encoding: "H264".to_string(),
            parameters: Some("profile-level-id=42e01f".to_string()),
        })
        .await;

    stream
        .add_track(TrackInfo {
            track_type: TrackType::Audio,
            payload_type: 97,
            clock_rate: 44100,
            encoding: "AAC".to_string(),
            parameters: None,
        })
        .await;

    let tracks = stream.tracks().await;
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0].encoding, "H264");
    assert_eq!(tracks[1].encoding, "AAC");
}

// =============================================================================
// Session Tests
// =============================================================================

#[tokio::test]
async fn test_create_session() {
    let router = Router::new();
    let session = router.create_session("session1").await.unwrap();
    assert_eq!(session.id, "session1");
    assert_eq!(session.state().await, SessionState::Init);
}

#[tokio::test]
async fn test_create_duplicate_session() {
    let router = Router::new();
    router.create_session("session1").await.unwrap();

    let result = router.create_session("session1").await;
    assert!(matches!(result, Err(CoreError::SessionExists(_))));
}

#[tokio::test]
async fn test_session_state() {
    let router = Router::new();
    let session = router.create_session("session1").await.unwrap();

    session.set_state(SessionState::Ready).await;
    assert_eq!(session.state().await, SessionState::Ready);

    session.set_state(SessionState::Playing).await;
    assert_eq!(session.state().await, SessionState::Playing);
}

#[tokio::test]
async fn test_session_mode() {
    let router = Router::new();
    let session = router.create_session("session1").await.unwrap();

    assert_eq!(session.mode().await, SessionMode::Play);
    session.set_mode(SessionMode::Record).await;
    assert_eq!(session.mode().await, SessionMode::Record);
}

#[tokio::test]
async fn test_remove_session() {
    let router = Router::new();
    router.create_session("session1").await.unwrap();

    router.remove_session("session1").await.unwrap();
    assert!(router.get_session("session1").await.is_none());
}

// =============================================================================
// Sink Tests
// =============================================================================

#[tokio::test]
async fn test_add_sink() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();

    let rx = router.add_sink("stream1", "sink1", None).await;
    assert!(rx.is_ok());
}

#[tokio::test]
async fn test_add_sink_nonexistent_stream() {
    let router = Router::new();

    let result = router.add_sink("nonexistent", "sink1", None).await;
    assert!(matches!(result, Err(CoreError::StreamNotFound(_))));
}

#[tokio::test]
async fn test_remove_sink() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();
    router.add_sink("stream1", "sink1", None).await.unwrap();

    router.remove_sink_from_stream("stream1", "sink1").await.unwrap();

    // Sink count should be 0
    let stream = router.get_stream("stream1").await.unwrap();
    assert_eq!(stream.sink_count().await, 0);
}

#[tokio::test]
async fn test_multiple_sinks() {
    let router = Router::new();
    let stream = router.create_stream("stream1").await.unwrap();

    router.add_sink("stream1", "sink1", None).await.unwrap();
    router.add_sink("stream1", "sink2", None).await.unwrap();
    router.add_sink("stream1", "sink3", None).await.unwrap();

    assert_eq!(stream.sink_count().await, 3);
}

// =============================================================================
// Source Tests
// =============================================================================

#[tokio::test]
async fn test_set_source() {
    let router = Router::new();
    let stream = router.create_stream("stream1").await.unwrap();

    router.set_source("stream1", "source1").await.unwrap();
    assert!(stream.has_source().await);
    assert_eq!(stream.source().await, Some("source1".to_string()));
    assert_eq!(stream.state().await, StreamState::Active);
}

#[tokio::test]
async fn test_duplicate_source() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();

    router.set_source("stream1", "source1").await.unwrap();
    let result = router.set_source("stream1", "source2").await;
    assert!(matches!(result, Err(CoreError::SourceExists(_))));
}

#[tokio::test]
async fn test_clear_source() {
    let router = Router::new();
    let stream = router.create_stream("stream1").await.unwrap();

    router.set_source("stream1", "source1").await.unwrap();
    router.clear_source("stream1").await.unwrap();

    assert!(!stream.has_source().await);
    assert_eq!(stream.state().await, StreamState::Idle);
}

// =============================================================================
// Packet Forwarding Tests
// =============================================================================

fn make_rtp_packet(seq: u16, pt: u8) -> Bytes {
    let mut data = vec![
        0x80,
        pt,          // V=2, P=0, X=0, CC=0, M=0, PT
        0x00, 0x00,  // Sequence number
        0x00, 0x00, 0x00, 0x00, // Timestamp
        0x00, 0x00, 0x00, 0x01, // SSRC
        0x01, 0x02, 0x03, 0x04, // Payload
    ];
    data[2] = (seq >> 8) as u8;
    data[3] = seq as u8;
    Bytes::from(data)
}

#[tokio::test]
async fn test_forward_packet() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();
    let mut rx = router.add_sink("stream1", "sink1", None).await.unwrap();
    router.set_source("stream1", "source1").await.unwrap();

    let packet = make_rtp_packet(1, 96);
    router.on_rtp("stream1", packet).await.unwrap();

    let received = rx.recv().await.unwrap();
    assert_eq!(received.sequence, 1);
    assert_eq!(received.payload_type, 96);
}

#[tokio::test]
async fn test_forward_to_multiple_sinks() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();

    let mut rx1 = router.add_sink("stream1", "sink1", None).await.unwrap();
    let mut rx2 = router.add_sink("stream1", "sink2", None).await.unwrap();
    let mut rx3 = router.add_sink("stream1", "sink3", None).await.unwrap();

    router.set_source("stream1", "source1").await.unwrap();

    let packet = make_rtp_packet(100, 96);
    router.on_rtp("stream1", packet).await.unwrap();

    // All three should receive the packet
    let p1 = rx1.recv().await.unwrap();
    let p2 = rx2.recv().await.unwrap();
    let p3 = rx3.recv().await.unwrap();

    assert_eq!(p1.sequence, 100);
    assert_eq!(p2.sequence, 100);
    assert_eq!(p3.sequence, 100);
}

#[tokio::test]
async fn test_no_sinks_drops_packet() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();
    router.set_source("stream1", "source1").await.unwrap();

    // No sinks, should not error
    let packet = make_rtp_packet(1, 96);
    let result = router.on_rtp("stream1", packet).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_forward_nonexistent_stream() {
    let router = Router::new();

    let packet = make_rtp_packet(1, 96);
    let result = router.on_rtp("nonexistent", packet).await;
    assert!(matches!(result, Err(CoreError::StreamNotFound(_))));
}

// =============================================================================
// Statistics Tests
// =============================================================================

#[tokio::test]
async fn test_stream_stats() {
    let router = Router::new();
    let stream = router.create_stream("stream1").await.unwrap();
    let _rx = router.add_sink("stream1", "sink1", None).await.unwrap();
    router.set_source("stream1", "source1").await.unwrap();

    // Send some packets
    for seq in 0..10 {
        let packet = make_rtp_packet(seq, 96);
        router.on_rtp("stream1", packet).await.unwrap();
    }

    let stats = stream.stats().snapshot();
    assert_eq!(stats.packets_received, 10);
    assert_eq!(stats.packets_forwarded, 10);
    assert_eq!(stats.packets_dropped, 0);
}

#[tokio::test]
async fn test_router_stats() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();
    router.create_stream("stream2").await.unwrap();
    router.create_session("session1").await.unwrap();
    router.set_source("stream1", "source1").await.unwrap();
    router.add_sink("stream1", "sink1", None).await.unwrap();
    router.add_sink("stream1", "sink2", None).await.unwrap();

    let stats = router.stats().await;
    assert_eq!(stats.stream_count, 2);
    assert_eq!(stats.session_count, 1);
    assert_eq!(stats.source_count, 1);
    assert_eq!(stats.sink_count, 2);
}

// =============================================================================
// MediaPacket Tests
// =============================================================================

#[test]
fn test_media_packet_from_rtp() {
    let data = Bytes::from(vec![
        0x80, 0xE0, // V=2, M=1, PT=96
        0x00, 0x0A, // Seq=10
        0x00, 0x01, 0x00, 0x00, // Timestamp=65536
        0x12, 0x34, 0x56, 0x78, // SSRC
        0x01, 0x02, 0x03,       // Payload
    ]);

    let packet = MediaPacket::from_rtp(data).unwrap();
    assert_eq!(packet.payload_type, 96);
    assert!(packet.marker);
    assert_eq!(packet.sequence, 10);
    assert_eq!(packet.timestamp, 65536);
    assert_eq!(packet.ssrc, 0x12345678);
}

#[test]
fn test_media_packet_too_short() {
    let data = Bytes::from(vec![0x80, 0x60, 0x00]); // Only 3 bytes
    assert!(MediaPacket::from_rtp(data).is_none());
}

// =============================================================================
// TrackId Tests
// =============================================================================

#[test]
fn test_track_id_video() {
    let track = TrackId::video("stream1");
    assert_eq!(track.stream_id, "stream1");
    assert_eq!(track.track_type, TrackType::Video);
    assert_eq!(track.index, 0);
}

#[test]
fn test_track_id_audio() {
    let track = TrackId::audio("stream1");
    assert_eq!(track.stream_id, "stream1");
    assert_eq!(track.track_type, TrackType::Audio);
    assert_eq!(track.index, 0);
}

// =============================================================================
// Sink Configuration Tests
// =============================================================================

#[tokio::test]
async fn test_custom_queue_size() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();

    let config = SinkConfig {
        queue_size: 10,
        queue_full_behavior: QueueFullBehavior::DropNewest,
    };

    let rx = router.add_sink("stream1", "sink1", Some(config)).await;
    assert!(rx.is_ok());
}

// =============================================================================
// DropOldest Queue Behavior Tests
// =============================================================================

#[tokio::test]
async fn test_drop_oldest_behavior() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();

    // Create sink with small queue and DropOldest behavior
    let config = SinkConfig {
        queue_size: 3,
        queue_full_behavior: QueueFullBehavior::DropOldest,
    };

    let mut rx = router.add_sink("stream1", "sink1", Some(config)).await.unwrap();
    router.set_source("stream1", "source1").await.unwrap();

    // Send 5 packets (queue size is 3)
    for seq in 1..=5u16 {
        let packet = make_rtp_packet(seq, 96);
        router.on_rtp("stream1", packet).await.unwrap();
    }

    // Should receive packets 3, 4, 5 (oldest 1, 2 were dropped)
    let p1 = rx.recv().await.unwrap();
    let p2 = rx.recv().await.unwrap();
    let p3 = rx.recv().await.unwrap();

    assert_eq!(p1.sequence, 3, "Expected packet 3, got {}", p1.sequence);
    assert_eq!(p2.sequence, 4, "Expected packet 4, got {}", p2.sequence);
    assert_eq!(p3.sequence, 5, "Expected packet 5, got {}", p3.sequence);
}

#[tokio::test]
async fn test_drop_oldest_stats() {
    let router = Router::new();
    let stream = router.create_stream("stream1").await.unwrap();

    let config = SinkConfig {
        queue_size: 2,
        queue_full_behavior: QueueFullBehavior::DropOldest,
    };

    let _rx = router.add_sink("stream1", "sink1", Some(config)).await.unwrap();
    router.set_source("stream1", "source1").await.unwrap();

    // Send 5 packets (queue size is 2, so 3 will be dropped)
    for seq in 1..=5u16 {
        let packet = make_rtp_packet(seq, 96);
        router.on_rtp("stream1", packet).await.unwrap();
    }

    let stats = stream.stats().snapshot();
    assert_eq!(stats.packets_received, 5);
    // With DropOldest ring buffer, try_send always succeeds (returns Ok),
    // so router sees all 5 as forwarded. The drops happen inside the sink.
    assert_eq!(stats.packets_forwarded, 5);
    // Stream stats don't see ring buffer internal drops
    assert_eq!(stats.packets_dropped, 0);
}

// =============================================================================
// Remove Stream with Sinks Tests
// =============================================================================

#[tokio::test]
async fn test_remove_stream_removes_sinks() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();
    router.add_sink("stream1", "sink1", None).await.unwrap();
    router.add_sink("stream1", "sink2", None).await.unwrap();

    let stats_before = router.stats().await;
    assert_eq!(stats_before.sink_count, 2);

    router.remove_stream("stream1").await.unwrap();

    let stats_after = router.stats().await;
    assert_eq!(stats_after.sink_count, 0);
    assert_eq!(stats_after.stream_count, 0);
}

// =============================================================================
// Session with Sinks Tests
// =============================================================================

#[tokio::test]
async fn test_session_tracks_sinks() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();
    let session = router.create_session("session1").await.unwrap();

    // Add sink and link to session
    router.add_sink("stream1", "sink1", None).await.unwrap();
    session.add_sink("sink1".to_string()).await;

    let sinks = session.sinks().await;
    assert_eq!(sinks, vec!["sink1"]);
}

#[tokio::test]
async fn test_remove_session_removes_sinks() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();
    let session = router.create_session("session1").await.unwrap();

    router.add_sink("stream1", "sink1", None).await.unwrap();
    session.add_sink("sink1".to_string()).await;

    let stats_before = router.stats().await;
    assert_eq!(stats_before.sink_count, 1);

    router.remove_session("session1").await.unwrap();

    let stats_after = router.stats().await;
    assert_eq!(stats_after.sink_count, 0);
    assert_eq!(stats_after.session_count, 0);
}

// =============================================================================
// Track-Level Attachment Tests
// =============================================================================

#[tokio::test]
async fn test_attach_track() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();
    router.set_source("stream1", "source1").await.unwrap();

    // Create a standalone sink and attach to payload type 96 (video)
    let mut rx = router.create_sink("sink1", "stream1", None).await.unwrap();
    router.attach_track("stream1", "sink1", 96, "attach1").await.unwrap();

    // Send video packet (PT 96) - should be received
    let video_packet = make_rtp_packet(1, 96);
    router.on_rtp("stream1", video_packet).await.unwrap();

    // Send audio packet (PT 97) - should NOT be received
    let audio_packet = make_rtp_packet(2, 97);
    router.on_rtp("stream1", audio_packet).await.unwrap();

    // Should only receive the video packet
    let packet = rx.try_recv().unwrap();
    assert_eq!(packet.payload_type, 96);
    assert_eq!(packet.sequence, 1);

    // Should have no more packets
    assert!(rx.try_recv().is_none());
}

#[tokio::test]
async fn test_attach_multiple_tracks() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();
    router.set_source("stream1", "source1").await.unwrap();

    // Create a sink and attach to both video (96) and audio (97)
    let mut rx = router.create_sink("sink1", "stream1", None).await.unwrap();
    router.attach_track("stream1", "sink1", 96, "attach_video").await.unwrap();
    router.attach_track("stream1", "sink1", 97, "attach_audio").await.unwrap();

    // Send video, audio, and data (98) packets
    router.on_rtp("stream1", make_rtp_packet(1, 96)).await.unwrap();
    router.on_rtp("stream1", make_rtp_packet(2, 97)).await.unwrap();
    router.on_rtp("stream1", make_rtp_packet(3, 98)).await.unwrap(); // data - not attached

    // Should receive video and audio only
    let p1 = rx.try_recv().unwrap();
    let p2 = rx.try_recv().unwrap();
    assert_eq!(p1.payload_type, 96);
    assert_eq!(p2.payload_type, 97);

    // No more packets (data packet wasn't attached)
    assert!(rx.try_recv().is_none());
}

#[tokio::test]
async fn test_detach_track() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();
    router.set_source("stream1", "source1").await.unwrap();

    let mut rx = router.create_sink("sink1", "stream1", None).await.unwrap();
    router.attach_track("stream1", "sink1", 96, "attach1").await.unwrap();

    // Send packet - should receive it
    router.on_rtp("stream1", make_rtp_packet(1, 96)).await.unwrap();
    assert!(rx.try_recv().is_some());

    // Detach
    router.detach_track("attach1").await.unwrap();

    // Send another packet - should NOT receive it
    router.on_rtp("stream1", make_rtp_packet(2, 96)).await.unwrap();
    assert!(rx.try_recv().is_none());
}

#[tokio::test]
async fn test_stream_sink_and_attachment_together() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();
    router.set_source("stream1", "source1").await.unwrap();

    // Stream-level sink (gets ALL packets)
    let mut stream_rx = router.add_sink("stream1", "stream_sink", None).await.unwrap();

    // Track-level sink (only video)
    let mut video_rx = router.create_sink("video_sink", "stream1", None).await.unwrap();
    router.attach_track("stream1", "video_sink", 96, "attach_video").await.unwrap();

    // Send video and audio
    router.on_rtp("stream1", make_rtp_packet(1, 96)).await.unwrap();
    router.on_rtp("stream1", make_rtp_packet(2, 97)).await.unwrap();

    // Stream sink gets both
    let p1 = stream_rx.try_recv().unwrap();
    let p2 = stream_rx.try_recv().unwrap();
    assert_eq!(p1.payload_type, 96);
    assert_eq!(p2.payload_type, 97);

    // Video sink gets only video
    let v1 = video_rx.try_recv().unwrap();
    assert_eq!(v1.payload_type, 96);
    assert!(video_rx.try_recv().is_none());
}

#[tokio::test]
async fn test_get_attachment() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();
    router.create_sink("sink1", "stream1", None).await.unwrap();
    router.attach_track("stream1", "sink1", 96, "attach1").await.unwrap();

    let attachment = router.get_attachment("attach1").await.unwrap();
    assert_eq!(attachment.id, "attach1");
    assert_eq!(attachment.sink_id, "sink1");
    assert!(attachment.active);
}

#[tokio::test]
async fn test_list_attachments() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();
    router.create_sink("sink1", "stream1", None).await.unwrap();
    router.attach_track("stream1", "sink1", 96, "attach_video").await.unwrap();
    router.attach_track("stream1", "sink1", 97, "attach_audio").await.unwrap();

    let attachments = router.list_attachments("stream1").await;
    assert_eq!(attachments.len(), 2);
}

#[tokio::test]
async fn test_remove_sink_removes_attachments() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();
    router.create_sink("sink1", "stream1", None).await.unwrap();
    router.attach_track("stream1", "sink1", 96, "attach1").await.unwrap();

    // Verify attachment exists
    assert!(router.get_attachment("attach1").await.is_some());

    // Remove sink
    router.destroy_sink("sink1").await.unwrap();

    // Attachment should be gone
    assert!(router.get_attachment("attach1").await.is_none());
}

#[tokio::test]
async fn test_remove_stream_removes_attachments() {
    let router = Router::new();
    router.create_stream("stream1").await.unwrap();
    router.create_sink("sink1", "stream1", None).await.unwrap();
    router.attach_track("stream1", "sink1", 96, "attach1").await.unwrap();

    // Verify attachment exists
    assert!(router.get_attachment("attach1").await.is_some());

    // Remove stream
    router.remove_stream("stream1").await.unwrap();

    // Attachment should be gone
    assert!(router.get_attachment("attach1").await.is_none());
}
