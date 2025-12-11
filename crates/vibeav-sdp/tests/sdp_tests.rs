//! SDP parser tests.

use vibeav_sdp::{
    parse, resolve_control_url, Connection, Direction, Fmtp, MediaDescription, Origin, Range,
    RtpMap, SdpError, Timing, SDP_VERSION,
};

// =============================================================================
// Basic Parsing Tests
// =============================================================================

#[test]
fn test_parse_minimal_sdp() {
    let sdp = "v=0\r\n\
               o=- 1234 1234 IN IP4 127.0.0.1\r\n\
               s=Test\r\n\
               t=0 0\r\n";

    let session = parse(sdp).unwrap();
    assert_eq!(session.version, SDP_VERSION);
    assert_eq!(session.name, "Test");
    assert!(session.origin.is_some());
    assert!(session.timing.is_some());
    assert!(session.media.is_empty());
}

#[test]
fn test_parse_with_single_media() {
    let sdp = "v=0\r\n\
               o=- 1234 1234 IN IP4 127.0.0.1\r\n\
               s=Test\r\n\
               t=0 0\r\n\
               m=video 0 RTP/AVP 96\r\n\
               a=rtpmap:96 H264/90000\r\n";

    let session = parse(sdp).unwrap();
    assert_eq!(session.media.len(), 1);
    let video = &session.media[0];
    assert_eq!(video.media_type, "video");
    assert_eq!(video.port, 0);
    assert_eq!(video.protocol, "RTP/AVP");
    assert!(video.is_video());
    assert!(!video.is_audio());
}

#[test]
fn test_parse_complete_rtsp_sdp() {
    let sdp = r#"v=0
o=- 2890844256 2890842807 IN IP4 192.168.1.100
s=Example Movie
i=A sample H.264/AAC movie
u=http://example.com/info.html
e=admin@example.com
c=IN IP4 0.0.0.0
t=0 0
a=recvonly
a=control:rtsp://example.com/movie/
a=range:npt=0-3600.0
a=etag:abc123def456
m=video 0 RTP/AVP 96
a=rtpmap:96 H264/90000
a=fmtp:96 profile-level-id=42e01f; packetization-mode=1
a=control:trackID=1
a=framerate:30
m=audio 0 RTP/AVP 97
a=rtpmap:97 mpeg4-generic/44100/2
a=fmtp:97 streamtype=5; profile-level-id=1; mode=AAC-hbr
a=control:trackID=2
"#;

    let session = parse(sdp).unwrap();

    // Session level
    assert_eq!(session.name, "Example Movie");
    assert_eq!(session.info.as_deref(), Some("A sample H.264/AAC movie"));
    assert_eq!(session.uri.as_deref(), Some("http://example.com/info.html"));
    assert_eq!(session.email.as_deref(), Some("admin@example.com"));
    assert_eq!(session.direction, Direction::RecvOnly);
    assert_eq!(
        session.control.as_deref(),
        Some("rtsp://example.com/movie/")
    );
    assert_eq!(session.etag.as_deref(), Some("abc123def456"));

    // Range
    assert!(session.range.is_some());
    if let Some(Range::Npt { start, end }) = &session.range {
        assert_eq!(*start, 0.0);
        assert_eq!(*end, Some(3600.0));
    } else {
        panic!("Expected NPT range");
    }

    // Media streams
    assert_eq!(session.media.len(), 2);

    // Video
    let video = &session.media[0];
    assert_eq!(video.media_type, "video");
    assert_eq!(video.encoding(), Some("H264"));
    assert_eq!(video.clock_rate(), Some(90000));
    assert_eq!(video.control.as_deref(), Some("trackID=1"));
    assert_eq!(video.framerate, Some(30.0));
    assert_eq!(video.primary_payload_type(), Some(96));

    // Video fmtp
    let fmtp = video.get_fmtp(96).unwrap();
    assert_eq!(fmtp.profile_level_id(), Some("42e01f"));
    assert_eq!(fmtp.packetization_mode(), Some(1));

    // Audio
    let audio = &session.media[1];
    assert_eq!(audio.media_type, "audio");
    assert_eq!(audio.encoding(), Some("mpeg4-generic"));
    assert_eq!(audio.clock_rate(), Some(44100));
    assert_eq!(audio.control.as_deref(), Some("trackID=2"));

    // Audio rtpmap channels
    let rtpmap = audio.get_rtpmap(97).unwrap();
    assert_eq!(rtpmap.channels(), Some(2));
}

// =============================================================================
// Origin Parsing
// =============================================================================

#[test]
fn test_origin_parse() {
    let origin = Origin::parse("- 2890844256 2890842807 IN IP4 192.168.1.100").unwrap();
    assert_eq!(origin.username, "-");
    assert_eq!(origin.session_id, "2890844256");
    assert_eq!(origin.session_version, "2890842807");
    assert_eq!(origin.net_type, "IN");
    assert_eq!(origin.addr_type, "IP4");
    assert_eq!(origin.address, "192.168.1.100");
}

#[test]
fn test_origin_parse_with_username() {
    let origin = Origin::parse("admin 1234 5678 IN IP6 ::1").unwrap();
    assert_eq!(origin.username, "admin");
    assert_eq!(origin.addr_type, "IP6");
    assert_eq!(origin.address, "::1");
}

#[test]
fn test_origin_parse_invalid() {
    assert!(Origin::parse("- 1234").is_none());
    assert!(Origin::parse("").is_none());
}

// =============================================================================
// Connection Parsing
// =============================================================================

#[test]
fn test_connection_parse_unicast() {
    let conn = Connection::parse("IN IP4 0.0.0.0").unwrap();
    assert_eq!(conn.net_type, "IN");
    assert_eq!(conn.addr_type, "IP4");
    assert_eq!(conn.address, "0.0.0.0");
    assert!(conn.ttl.is_none());
    assert!(!conn.is_multicast());
}

#[test]
fn test_connection_parse_multicast_with_ttl() {
    let conn = Connection::parse("IN IP4 224.2.0.1/16").unwrap();
    assert_eq!(conn.address, "224.2.0.1");
    assert_eq!(conn.ttl, Some(16));
    assert!(conn.is_multicast());
}

#[test]
fn test_connection_parse_multicast_with_count() {
    let conn = Connection::parse("IN IP4 224.2.0.1/16/3").unwrap();
    assert_eq!(conn.ttl, Some(16));
    assert_eq!(conn.num_addresses, Some(3));
}

#[test]
fn test_connection_ipv6_multicast() {
    let conn = Connection::parse("IN IP6 ff00::1").unwrap();
    assert!(conn.is_multicast());
}

#[test]
fn test_connection_ipv6_unicast() {
    let conn = Connection::parse("IN IP6 2001:db8::1").unwrap();
    assert!(!conn.is_multicast());
}

// =============================================================================
// Timing Parsing
// =============================================================================

#[test]
fn test_timing_permanent() {
    let timing = Timing::parse("0 0").unwrap();
    assert_eq!(timing.start, 0);
    assert_eq!(timing.stop, 0);
    assert!(timing.is_permanent());
}

#[test]
fn test_timing_bounded() {
    let timing = Timing::parse("3034423619 3042462419").unwrap();
    assert_eq!(timing.start, 3034423619);
    assert_eq!(timing.stop, 3042462419);
    assert!(!timing.is_permanent());
}

#[test]
fn test_timing_invalid() {
    assert!(Timing::parse("0").is_none());
    assert!(Timing::parse("abc def").is_none());
}

// =============================================================================
// Direction Parsing
// =============================================================================

#[test]
fn test_direction_from_attr() {
    assert_eq!(Direction::from_attr("sendrecv"), Some(Direction::SendRecv));
    assert_eq!(Direction::from_attr("recvonly"), Some(Direction::RecvOnly));
    assert_eq!(Direction::from_attr("sendonly"), Some(Direction::SendOnly));
    assert_eq!(Direction::from_attr("inactive"), Some(Direction::Inactive));
    assert_eq!(Direction::from_attr("unknown"), None);
}

#[test]
fn test_default_direction() {
    let sdp = "v=0\r\n\
               o=- 1234 1234 IN IP4 127.0.0.1\r\n\
               s=Test\r\n\
               t=0 0\r\n";

    let session = parse(sdp).unwrap();
    assert_eq!(session.direction, Direction::SendRecv);
}

// =============================================================================
// Range Parsing
// =============================================================================

#[test]
fn test_range_npt_bounded() {
    let range = Range::parse("npt=0-634.10").unwrap();
    if let Range::Npt { start, end } = range {
        assert_eq!(start, 0.0);
        assert_eq!(end, Some(634.10));
    } else {
        panic!("Expected NPT range");
    }
}

#[test]
fn test_range_npt_unbounded() {
    let range = Range::parse("npt=0-").unwrap();
    if let Range::Npt { start, end } = range {
        assert_eq!(start, 0.0);
        assert_eq!(end, None);
    } else {
        panic!("Expected NPT range");
    }
}

#[test]
fn test_range_smpte() {
    let range = Range::parse("smpte=0:0:0-1:30:0").unwrap();
    if let Range::Smpte { start, end } = range {
        assert_eq!(start, "0:0:0");
        assert_eq!(end, Some("1:30:0".to_string()));
    } else {
        panic!("Expected SMPTE range");
    }
}

#[test]
fn test_range_clock() {
    let range = Range::parse("clock=19970113T2115-").unwrap();
    if let Range::Clock { start, end } = range {
        assert_eq!(start, "19970113T2115");
        assert!(end.is_none());
    } else {
        panic!("Expected clock range");
    }
}

#[test]
fn test_range_duration() {
    let range = Range::parse("npt=10-60").unwrap();
    assert_eq!(range.duration(), Some(50.0));

    let unbounded = Range::parse("npt=0-").unwrap();
    assert_eq!(unbounded.duration(), None);

    let smpte = Range::parse("smpte=0:0:0-1:0:0").unwrap();
    assert_eq!(smpte.duration(), None);
}

// =============================================================================
// RtpMap Parsing
// =============================================================================

#[test]
fn test_rtpmap_video() {
    let rtpmap = RtpMap::parse("96 H264/90000").unwrap();
    assert_eq!(rtpmap.payload_type, 96);
    assert_eq!(rtpmap.encoding, "H264");
    assert_eq!(rtpmap.clock_rate, 90000);
    assert!(rtpmap.encoding_params.is_none());
}

#[test]
fn test_rtpmap_audio_stereo() {
    let rtpmap = RtpMap::parse("97 mpeg4-generic/44100/2").unwrap();
    assert_eq!(rtpmap.payload_type, 97);
    assert_eq!(rtpmap.encoding, "mpeg4-generic");
    assert_eq!(rtpmap.clock_rate, 44100);
    assert_eq!(rtpmap.encoding_params, Some("2".to_string()));
    assert_eq!(rtpmap.channels(), Some(2));
}

#[test]
fn test_rtpmap_audio_mono() {
    let rtpmap = RtpMap::parse("0 PCMU/8000/1").unwrap();
    assert_eq!(rtpmap.payload_type, 0);
    assert_eq!(rtpmap.encoding, "PCMU");
    assert_eq!(rtpmap.clock_rate, 8000);
    assert_eq!(rtpmap.channels(), Some(1));
}

#[test]
fn test_rtpmap_invalid() {
    assert!(RtpMap::parse("notanumber H264/90000").is_none());
    assert!(RtpMap::parse("96").is_none());
}

// =============================================================================
// Fmtp Parsing
// =============================================================================

#[test]
fn test_fmtp_h264() {
    let fmtp = Fmtp::parse("96 profile-level-id=42e01f; packetization-mode=1; sprop-parameter-sets=Z0IAH5WoFAFu,aM4G4g==").unwrap();
    assert_eq!(fmtp.payload_type, 96);
    assert_eq!(fmtp.profile_level_id(), Some("42e01f"));
    assert_eq!(fmtp.packetization_mode(), Some(1));
    assert_eq!(
        fmtp.sprop_parameter_sets(),
        Some("Z0IAH5WoFAFu,aM4G4g==")
    );
}

#[test]
fn test_fmtp_aac() {
    let fmtp = Fmtp::parse("97 streamtype=5; profile-level-id=1; mode=AAC-hbr").unwrap();
    assert_eq!(fmtp.get_param("streamtype"), Some("5"));
    assert_eq!(fmtp.get_param("mode"), Some("AAC-hbr"));
}

#[test]
fn test_fmtp_params_iterator() {
    let fmtp = Fmtp::parse("96 a=1; b=2; c=3").unwrap();
    let params: Vec<_> = fmtp.params().collect();
    assert_eq!(params.len(), 3);
    assert!(params.contains(&("a", "1")));
    assert!(params.contains(&("b", "2")));
    assert!(params.contains(&("c", "3")));
}

#[test]
fn test_fmtp_case_insensitive_lookup() {
    let fmtp = Fmtp::parse("96 Profile-Level-ID=42e01f").unwrap();
    assert_eq!(fmtp.get_param("profile-level-id"), Some("42e01f"));
}

// =============================================================================
// Media Description
// =============================================================================

#[test]
fn test_media_parse_video() {
    let media = MediaDescription::parse_media_line("video 0 RTP/AVP 96 97").unwrap();
    assert_eq!(media.media_type, "video");
    assert_eq!(media.port, 0);
    assert_eq!(media.protocol, "RTP/AVP");
    assert_eq!(media.formats, vec!["96", "97"]);
    assert!(media.is_video());
    assert!(media.is_rtp());
}

#[test]
fn test_media_parse_audio() {
    let media = MediaDescription::parse_media_line("audio 49170 RTP/AVP 0").unwrap();
    assert_eq!(media.media_type, "audio");
    assert_eq!(media.port, 49170);
    assert!(media.is_audio());
}

#[test]
fn test_media_parse_with_port_count() {
    let media = MediaDescription::parse_media_line("video 3456/2 RTP/AVP 96").unwrap();
    assert_eq!(media.port, 3456);
    assert_eq!(media.num_ports, Some(2));
}

#[test]
fn test_media_parse_invalid() {
    assert!(MediaDescription::parse_media_line("video").is_none());
    assert!(MediaDescription::parse_media_line("video 0").is_none());
    assert!(MediaDescription::parse_media_line("video 0 RTP/AVP").is_none());
}

// =============================================================================
// Control URL Resolution
// =============================================================================

#[test]
fn test_resolve_control_asterisk() {
    assert_eq!(
        resolve_control_url("rtsp://example.com/movie/", "*"),
        "rtsp://example.com/movie/"
    );
}

#[test]
fn test_resolve_control_absolute() {
    assert_eq!(
        resolve_control_url("rtsp://example.com/movie/", "rtsp://other.com/stream"),
        "rtsp://other.com/stream"
    );
}

#[test]
fn test_resolve_control_absolute_rtsps() {
    assert_eq!(
        resolve_control_url("rtsp://example.com/", "rtsps://secure.com/stream"),
        "rtsps://secure.com/stream"
    );
}

#[test]
fn test_resolve_control_relative() {
    assert_eq!(
        resolve_control_url("rtsp://example.com/movie/", "trackID=1"),
        "rtsp://example.com/movie/trackID=1"
    );
}

#[test]
fn test_resolve_control_relative_no_trailing_slash() {
    assert_eq!(
        resolve_control_url("rtsp://example.com/movie", "trackID=1"),
        "rtsp://example.com/movie/trackID=1"
    );
}

// =============================================================================
// Error Cases
// =============================================================================

#[test]
fn test_error_empty_sdp() {
    let err = parse("").unwrap_err();
    assert!(matches!(err, SdpError::Empty));
}

#[test]
fn test_error_missing_version() {
    let sdp = "o=- 1234 1234 IN IP4 127.0.0.1\r\n\
               s=Test\r\n\
               t=0 0\r\n";

    let err = parse(sdp).unwrap_err();
    assert!(matches!(err, SdpError::MissingVersion));
}

#[test]
fn test_error_invalid_version() {
    let sdp = "v=1\r\n\
               o=- 1234 1234 IN IP4 127.0.0.1\r\n\
               s=Test\r\n\
               t=0 0\r\n";

    let err = parse(sdp).unwrap_err();
    assert!(matches!(err, SdpError::InvalidVersion(1)));
}

#[test]
fn test_error_missing_origin() {
    let sdp = "v=0\r\n\
               s=Test\r\n\
               t=0 0\r\n";

    let err = parse(sdp).unwrap_err();
    assert!(matches!(err, SdpError::MissingOrigin));
}

#[test]
fn test_error_missing_session_name() {
    let sdp = "v=0\r\n\
               o=- 1234 1234 IN IP4 127.0.0.1\r\n\
               t=0 0\r\n";

    let err = parse(sdp).unwrap_err();
    assert!(matches!(err, SdpError::MissingSessionName));
}

#[test]
fn test_error_invalid_line_format() {
    let sdp = "v=0\r\n\
               o=- 1234 1234 IN IP4 127.0.0.1\r\n\
               s=Test\r\n\
               invalid line\r\n\
               t=0 0\r\n";

    let err = parse(sdp).unwrap_err();
    assert!(matches!(err, SdpError::InvalidLine { .. }));
}

#[test]
fn test_error_invalid_origin() {
    let sdp = "v=0\r\n\
               o=invalid\r\n\
               s=Test\r\n\
               t=0 0\r\n";

    let err = parse(sdp).unwrap_err();
    assert!(matches!(err, SdpError::InvalidOrigin(_)));
}

#[test]
fn test_error_invalid_media() {
    let sdp = "v=0\r\n\
               o=- 1234 1234 IN IP4 127.0.0.1\r\n\
               s=Test\r\n\
               t=0 0\r\n\
               m=video\r\n";

    let err = parse(sdp).unwrap_err();
    assert!(matches!(err, SdpError::InvalidMedia(_)));
}

// =============================================================================
// Session Methods
// =============================================================================

#[test]
fn test_session_aggregate_control() {
    let sdp = "v=0\r\n\
               o=- 1234 1234 IN IP4 127.0.0.1\r\n\
               s=Test\r\n\
               t=0 0\r\n\
               a=control:rtsp://example.com/movie/\r\n\
               m=video 0 RTP/AVP 96\r\n\
               a=control:trackID=1\r\n";

    let session = parse(sdp).unwrap();
    assert!(session.has_aggregate_control());
    assert_eq!(
        session.aggregate_control(),
        Some("rtsp://example.com/movie/")
    );
}

#[test]
fn test_session_no_aggregate_control() {
    let sdp = "v=0\r\n\
               o=- 1234 1234 IN IP4 127.0.0.1\r\n\
               s=Test\r\n\
               t=0 0\r\n\
               m=video 0 RTP/AVP 96\r\n\
               a=control:rtsp://video.example.com/track1\r\n\
               m=audio 0 RTP/AVP 97\r\n\
               a=control:rtsp://audio.example.com/track2\r\n";

    let session = parse(sdp).unwrap();
    assert!(!session.has_aggregate_control());
    assert_eq!(session.aggregate_control(), None);
}

#[test]
fn test_session_video_audio_streams() {
    let sdp = "v=0\r\n\
               o=- 1234 1234 IN IP4 127.0.0.1\r\n\
               s=Test\r\n\
               t=0 0\r\n\
               m=video 0 RTP/AVP 96\r\n\
               m=audio 0 RTP/AVP 97\r\n\
               m=video 0 RTP/AVP 98\r\n\
               m=text 0 RTP/AVP 99\r\n";

    let session = parse(sdp).unwrap();
    assert_eq!(session.streams().len(), 4);
    assert_eq!(session.video_streams().count(), 2);
    assert_eq!(session.audio_streams().count(), 1);
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_unix_line_endings() {
    let sdp = "v=0\n\
               o=- 1234 1234 IN IP4 127.0.0.1\n\
               s=Test\n\
               t=0 0\n";

    let session = parse(sdp).unwrap();
    assert_eq!(session.name, "Test");
}

#[test]
fn test_empty_lines_ignored() {
    let sdp = "v=0\r\n\
               \r\n\
               o=- 1234 1234 IN IP4 127.0.0.1\r\n\
               \r\n\
               s=Test\r\n\
               t=0 0\r\n";

    let session = parse(sdp).unwrap();
    assert_eq!(session.name, "Test");
}

#[test]
fn test_unknown_line_types_ignored() {
    let sdp = "v=0\r\n\
               o=- 1234 1234 IN IP4 127.0.0.1\r\n\
               s=Test\r\n\
               x=custom data\r\n\
               t=0 0\r\n";

    let session = parse(sdp).unwrap();
    assert_eq!(session.name, "Test");
}

#[test]
fn test_bandwidth_ignored() {
    let sdp = "v=0\r\n\
               o=- 1234 1234 IN IP4 127.0.0.1\r\n\
               s=Test\r\n\
               b=AS:1024\r\n\
               t=0 0\r\n";

    let session = parse(sdp).unwrap();
    assert_eq!(session.name, "Test");
}

#[test]
fn test_unknown_attributes_stored() {
    let sdp = "v=0\r\n\
               o=- 1234 1234 IN IP4 127.0.0.1\r\n\
               s=Test\r\n\
               t=0 0\r\n\
               a=custom:value\r\n\
               a=flag\r\n\
               m=video 0 RTP/AVP 96\r\n\
               a=x-custom:media-value\r\n";

    let session = parse(sdp).unwrap();
    assert_eq!(session.attributes.len(), 2);
    assert!(session.attributes.contains(&("custom".to_string(), Some("value".to_string()))));
    assert!(session.attributes.contains(&("flag".to_string(), None)));

    let video = &session.media[0];
    assert_eq!(video.attributes.len(), 1);
    assert!(video.attributes.contains(&("x-custom".to_string(), Some("media-value".to_string()))));
}

#[test]
fn test_media_inherits_connection() {
    let sdp = "v=0\r\n\
               o=- 1234 1234 IN IP4 127.0.0.1\r\n\
               s=Test\r\n\
               c=IN IP4 224.2.0.1/16\r\n\
               t=0 0\r\n\
               m=video 0 RTP/AVP 96\r\n";

    let session = parse(sdp).unwrap();
    assert!(session.connection.is_some());
    // Media-level connection is None (inherits from session)
    assert!(session.media[0].connection.is_none());
}

#[test]
fn test_media_overrides_connection() {
    let sdp = "v=0\r\n\
               o=- 1234 1234 IN IP4 127.0.0.1\r\n\
               s=Test\r\n\
               c=IN IP4 0.0.0.0\r\n\
               t=0 0\r\n\
               m=video 0 RTP/AVP 96\r\n\
               c=IN IP4 224.2.0.1/16\r\n";

    let session = parse(sdp).unwrap();
    let video = &session.media[0];
    assert!(video.connection.is_some());
    assert!(video.connection.as_ref().unwrap().is_multicast());
}

#[test]
fn test_multiple_rtpmap_fmtp() {
    let sdp = "v=0\r\n\
               o=- 1234 1234 IN IP4 127.0.0.1\r\n\
               s=Test\r\n\
               t=0 0\r\n\
               m=video 0 RTP/AVP 96 97 98\r\n\
               a=rtpmap:96 H264/90000\r\n\
               a=rtpmap:97 H265/90000\r\n\
               a=rtpmap:98 VP9/90000\r\n\
               a=fmtp:96 profile-level-id=42e01f\r\n\
               a=fmtp:97 profile-id=1\r\n";

    let session = parse(sdp).unwrap();
    let video = &session.media[0];

    assert_eq!(video.rtpmap.len(), 3);
    assert_eq!(video.fmtp.len(), 2);

    assert_eq!(video.get_rtpmap(96).unwrap().encoding, "H264");
    assert_eq!(video.get_rtpmap(97).unwrap().encoding, "H265");
    assert_eq!(video.get_rtpmap(98).unwrap().encoding, "VP9");

    assert_eq!(video.get_fmtp(96).unwrap().profile_level_id(), Some("42e01f"));
    assert_eq!(video.get_fmtp(97).unwrap().get_param("profile-id"), Some("1"));
    assert!(video.get_fmtp(98).is_none());
}

#[test]
fn test_media_direction() {
    let sdp = "v=0\r\n\
               o=- 1234 1234 IN IP4 127.0.0.1\r\n\
               s=Test\r\n\
               t=0 0\r\n\
               a=sendonly\r\n\
               m=video 0 RTP/AVP 96\r\n\
               a=recvonly\r\n\
               m=audio 0 RTP/AVP 97\r\n";

    let session = parse(sdp).unwrap();
    assert_eq!(session.direction, Direction::SendOnly);

    let video = &session.media[0];
    assert_eq!(video.direction, Some(Direction::RecvOnly));

    let audio = &session.media[1];
    assert!(audio.direction.is_none()); // inherits from session
}
