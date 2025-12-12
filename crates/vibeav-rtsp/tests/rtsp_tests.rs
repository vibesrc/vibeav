//! RTSP protocol tests.

use vibeav_rtsp::{
    CastMode, Headers, LowerTransport, Method, Request, Response, RtspError, StatusCode,
    Transport, TransportMode,
};

// =============================================================================
// Method Tests
// =============================================================================

#[test]
fn test_method_from_str() {
    assert_eq!("OPTIONS".parse::<Method>().unwrap(), Method::Options);
    assert_eq!("DESCRIBE".parse::<Method>().unwrap(), Method::Describe);
    assert_eq!("ANNOUNCE".parse::<Method>().unwrap(), Method::Announce);
    assert_eq!("SETUP".parse::<Method>().unwrap(), Method::Setup);
    assert_eq!("PLAY".parse::<Method>().unwrap(), Method::Play);
    assert_eq!("PAUSE".parse::<Method>().unwrap(), Method::Pause);
    assert_eq!("RECORD".parse::<Method>().unwrap(), Method::Record);
    assert_eq!("TEARDOWN".parse::<Method>().unwrap(), Method::Teardown);
    assert_eq!("GET_PARAMETER".parse::<Method>().unwrap(), Method::GetParameter);
    assert_eq!("SET_PARAMETER".parse::<Method>().unwrap(), Method::SetParameter);
}

#[test]
fn test_method_case_insensitive() {
    assert_eq!("options".parse::<Method>().unwrap(), Method::Options);
    assert_eq!("Options".parse::<Method>().unwrap(), Method::Options);
    assert_eq!("OPTIONS".parse::<Method>().unwrap(), Method::Options);
}

#[test]
fn test_method_invalid() {
    assert!("INVALID".parse::<Method>().is_err());
    assert!("".parse::<Method>().is_err());
}

#[test]
fn test_method_requires_session() {
    assert!(!Method::Options.requires_session());
    assert!(!Method::Describe.requires_session());
    assert!(!Method::Announce.requires_session());
    assert!(!Method::Setup.requires_session());
    assert!(Method::Play.requires_session());
    assert!(Method::Pause.requires_session());
    assert!(Method::Record.requires_session());
    assert!(Method::Teardown.requires_session());
}

// =============================================================================
// Status Code Tests
// =============================================================================

#[test]
fn test_status_code_success() {
    assert!(StatusCode::OK.is_success());
    assert!(StatusCode::CREATED.is_success());
    assert!(!StatusCode::NOT_FOUND.is_success());
}

#[test]
fn test_status_code_error() {
    assert!(StatusCode::BAD_REQUEST.is_client_error());
    assert!(StatusCode::NOT_FOUND.is_client_error());
    assert!(StatusCode::SESSION_NOT_FOUND.is_client_error());
    assert!(StatusCode::INTERNAL_SERVER_ERROR.is_server_error());
    assert!(StatusCode::NOT_IMPLEMENTED.is_server_error());
}

#[test]
fn test_status_code_rtsp_specific() {
    assert_eq!(StatusCode::SESSION_NOT_FOUND.as_u16(), 454);
    assert_eq!(StatusCode::UNSUPPORTED_TRANSPORT.as_u16(), 461);
    assert_eq!(StatusCode::METHOD_NOT_VALID.as_u16(), 455);
}

#[test]
fn test_status_code_reason() {
    assert_eq!(StatusCode::OK.reason(), "OK");
    assert_eq!(StatusCode::NOT_FOUND.reason(), "Not Found");
    assert_eq!(StatusCode::SESSION_NOT_FOUND.reason(), "Session Not Found");
    assert_eq!(StatusCode::UNSUPPORTED_TRANSPORT.reason(), "Unsupported Transport");
}

// =============================================================================
// Headers Tests
// =============================================================================

#[test]
fn test_headers_case_insensitive() {
    let mut headers = Headers::new();
    headers.insert("Content-Type", "application/sdp");
    headers.insert("CSeq", "1");

    assert_eq!(headers.get("content-type"), Some("application/sdp"));
    assert_eq!(headers.get("Content-Type"), Some("application/sdp"));
    assert_eq!(headers.get("CONTENT-TYPE"), Some("application/sdp"));
}

#[test]
fn test_headers_cseq() {
    let mut headers = Headers::new();
    headers.insert("CSeq", "42");
    assert_eq!(headers.cseq(), Some(42));
}

#[test]
fn test_headers_session() {
    let mut headers = Headers::new();

    headers.insert("Session", "12345678");
    assert_eq!(headers.session(), Some("12345678"));

    headers.insert("Session", "87654321;timeout=60");
    assert_eq!(headers.session(), Some("87654321"));
    assert_eq!(headers.session_timeout(), Some(60));
}

#[test]
fn test_headers_content_length() {
    let mut headers = Headers::new();
    headers.insert("Content-Length", "256");
    assert_eq!(headers.content_length(), Some(256));
}

#[test]
fn test_headers_public_methods() {
    let mut headers = Headers::new();
    headers.insert("Public", "DESCRIBE, SETUP, PLAY, TEARDOWN");

    let methods = headers.public_methods().unwrap();
    assert_eq!(methods, vec!["DESCRIBE", "SETUP", "PLAY", "TEARDOWN"]);
}

// =============================================================================
// Request Tests
// =============================================================================

#[test]
fn test_request_options() {
    let req = Request::options("rtsp://example.com/*").with_cseq(1);
    assert_eq!(req.method, Method::Options);
    assert_eq!(req.uri, "rtsp://example.com/*");
    assert_eq!(req.cseq(), Some(1));
}

#[test]
fn test_request_describe() {
    let req = Request::describe("rtsp://example.com/movie").with_cseq(2);
    assert_eq!(req.method, Method::Describe);
    assert_eq!(req.headers.accept(), Some("application/sdp"));
}

#[test]
fn test_request_setup() {
    let req = Request::setup_with_transport(
        "rtsp://example.com/movie/trackID=1",
        "RTP/AVP/TCP;unicast;interleaved=0-1",
    )
    .with_cseq(3);

    assert_eq!(req.method, Method::Setup);
    assert!(req.headers.transport().is_some());
}

#[test]
fn test_request_play() {
    let req = Request::play_with_session("rtsp://example.com/movie", "12345678").with_cseq(4);
    assert_eq!(req.method, Method::Play);
    assert_eq!(req.session(), Some("12345678"));
}

#[test]
fn test_request_record() {
    let req = Request::record_with_session("rtsp://example.com/stream", "12345678").with_cseq(5);
    assert_eq!(req.method, Method::Record);
    assert_eq!(req.session(), Some("12345678"));
}

#[test]
fn test_request_teardown() {
    let req = Request::teardown_with_session("rtsp://example.com/movie", "12345678").with_cseq(6);
    assert_eq!(req.method, Method::Teardown);
}

#[test]
fn test_request_announce() {
    let sdp = b"v=0\r\no=- 1234 1234 IN IP4 127.0.0.1\r\ns=Test\r\n";
    let req = Request::announce("rtsp://example.com/stream", sdp.to_vec()).with_cseq(3);

    assert_eq!(req.method, Method::Announce);
    assert_eq!(req.headers.content_type(), Some("application/sdp"));
    assert_eq!(req.headers.content_length(), Some(sdp.len()));
    assert_eq!(req.body, Some(sdp.to_vec()));
}

#[test]
fn test_request_parse() {
    let data = b"DESCRIBE rtsp://example.com/movie RTSP/1.0\r\n\
                 CSeq: 2\r\n\
                 Accept: application/sdp\r\n\
                 User-Agent: vibeav/1.0\r\n\
                 \r\n";

    let (req, consumed) = Request::parse(data).unwrap();
    assert_eq!(req.method, Method::Describe);
    assert_eq!(req.uri, "rtsp://example.com/movie");
    assert_eq!(req.cseq(), Some(2));
    assert_eq!(req.headers.accept(), Some("application/sdp"));
    assert_eq!(req.headers.user_agent(), Some("vibeav/1.0"));
    assert_eq!(consumed, data.len());
}

#[test]
fn test_request_parse_with_body() {
    let body = b"v=0\r\no=- 1234 1234 IN IP4 127.0.0.1\r\ns=Test\r\n";
    let data = format!(
        "ANNOUNCE rtsp://example.com/stream RTSP/1.0\r\n\
         CSeq: 3\r\n\
         Content-Type: application/sdp\r\n\
         Content-Length: {}\r\n\
         \r\n{}",
        body.len(),
        std::str::from_utf8(body).unwrap()
    );

    let (req, _) = Request::parse(data.as_bytes()).unwrap();
    assert_eq!(req.method, Method::Announce);
    assert_eq!(req.body, Some(body.to_vec()));
}

#[test]
fn test_request_parse_incomplete() {
    let data = b"DESCRIBE rtsp://example.com/movie RTSP/1.0\r\n\
                 CSeq: 2\r\n";

    assert!(matches!(Request::parse(data), Err(RtspError::Incomplete)));
}

#[test]
fn test_request_serialize() {
    let req = Request::describe("rtsp://example.com/movie")
        .with_cseq(1)
        .with_user_agent("vibeav/1.0");

    let bytes = req.to_bytes();
    let text = String::from_utf8(bytes).unwrap();

    assert!(text.starts_with("DESCRIBE rtsp://example.com/movie RTSP/1.0\r\n"));
    // Headers are lowercase-normalized in storage, capitalized on output
    assert!(text.to_lowercase().contains("cseq: 1"));
    assert!(text.to_lowercase().contains("accept: application/sdp"));
    assert!(text.to_lowercase().contains("user-agent: vibeav/1.0"));
    assert!(text.ends_with("\r\n\r\n"));
}

// =============================================================================
// Response Tests
// =============================================================================

#[test]
fn test_response_ok() {
    let resp = Response::ok().with_cseq(1);
    assert_eq!(resp.status, StatusCode::OK);
    assert!(resp.is_success());
    assert!(!resp.is_error());
}

#[test]
fn test_response_error() {
    let resp = Response::not_found().with_cseq(1);
    assert_eq!(resp.status, StatusCode::NOT_FOUND);
    assert!(resp.is_error());
    assert!(!resp.is_success());
}

#[test]
fn test_response_with_session() {
    let resp = Response::ok().with_session("12345678");
    assert_eq!(resp.session(), Some("12345678"));
}

#[test]
fn test_response_with_session_timeout() {
    let resp = Response::ok().with_session_timeout("12345678", 60);
    let session_header = resp.headers.get("session").unwrap();
    assert!(session_header.contains("12345678"));
    assert!(session_header.contains("timeout=60"));
}

#[test]
fn test_response_with_public() {
    let resp = Response::ok().with_public(&["DESCRIBE", "SETUP", "PLAY", "TEARDOWN"]);
    let methods = resp.headers.public_methods().unwrap();
    assert_eq!(methods.len(), 4);
}

#[test]
fn test_response_with_sdp() {
    let sdp = b"v=0\r\no=- 1234 1234 IN IP4 127.0.0.1\r\ns=Test\r\n";
    let resp = Response::ok()
        .with_cseq(1)
        .with_sdp(sdp.to_vec());

    assert_eq!(resp.headers.content_type(), Some("application/sdp"));
    assert_eq!(resp.headers.content_length(), Some(sdp.len()));
    assert_eq!(resp.body, Some(sdp.to_vec()));
}

#[test]
fn test_response_parse() {
    let data = b"RTSP/1.0 200 OK\r\n\
                 CSeq: 1\r\n\
                 Public: DESCRIBE, SETUP, PLAY, TEARDOWN\r\n\
                 Server: vibeav/1.0\r\n\
                 \r\n";

    let (resp, consumed) = Response::parse(data).unwrap();
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.cseq(), Some(1));
    assert!(resp.headers.public_methods().is_some());
    assert_eq!(resp.headers.server(), Some("vibeav/1.0"));
    assert_eq!(consumed, data.len());
}

#[test]
fn test_response_parse_with_body() {
    let body = b"v=0\r\no=- 1234 1234 IN IP4 127.0.0.1\r\ns=Test\r\n";
    let data = format!(
        "RTSP/1.0 200 OK\r\n\
         CSeq: 2\r\n\
         Content-Type: application/sdp\r\n\
         Content-Length: {}\r\n\
         \r\n{}",
        body.len(),
        std::str::from_utf8(body).unwrap()
    );

    let (resp, _) = Response::parse(data.as_bytes()).unwrap();
    assert!(resp.is_success());
    assert_eq!(resp.body, Some(body.to_vec()));
}

#[test]
fn test_response_serialize() {
    let resp = Response::ok()
        .with_cseq(1)
        .with_public(&["DESCRIBE", "SETUP", "PLAY"])
        .with_server("vibeav/1.0");

    let bytes = resp.to_bytes();
    let text = String::from_utf8(bytes).unwrap();

    assert!(text.starts_with("RTSP/1.0 200 OK\r\n"));
    // Headers are lowercase-normalized in storage, capitalized on output
    assert!(text.to_lowercase().contains("cseq: 1"));
    assert!(text.to_lowercase().contains("public: describe, setup, play"));
    assert!(text.to_lowercase().contains("server: vibeav/1.0"));
}

// =============================================================================
// Transport Tests
// =============================================================================

#[test]
fn test_transport_tcp_interleaved() {
    let t = Transport::tcp_interleaved(0, 1);
    assert_eq!(t.lower_transport, LowerTransport::Tcp);
    assert_eq!(t.cast_mode, CastMode::Unicast);
    assert_eq!(t.interleaved, Some((0, 1)));
    assert!(t.is_interleaved());
    assert!(!t.is_udp());
}

#[test]
fn test_transport_udp_unicast() {
    let t = Transport::udp_unicast(4588, 4589);
    assert_eq!(t.lower_transport, LowerTransport::Udp);
    assert_eq!(t.cast_mode, CastMode::Unicast);
    assert_eq!(t.client_port, Some((4588, 4589)));
    assert!(!t.is_interleaved());
    assert!(t.is_udp());
}

#[test]
fn test_transport_parse_tcp_interleaved() {
    let t = Transport::parse("RTP/AVP/TCP;unicast;interleaved=0-1").unwrap();
    assert_eq!(t.lower_transport, LowerTransport::Tcp);
    assert_eq!(t.interleaved, Some((0, 1)));
}

#[test]
fn test_transport_parse_udp_unicast() {
    let t = Transport::parse("RTP/AVP;unicast;client_port=4588-4589").unwrap();
    assert_eq!(t.lower_transport, LowerTransport::Udp);
    assert_eq!(t.client_port, Some((4588, 4589)));
}

#[test]
fn test_transport_parse_with_server_port() {
    let t = Transport::parse("RTP/AVP;unicast;client_port=4588-4589;server_port=6256-6257").unwrap();
    assert_eq!(t.client_port, Some((4588, 4589)));
    assert_eq!(t.server_port, Some((6256, 6257)));
}

#[test]
fn test_transport_parse_multicast() {
    let t = Transport::parse("RTP/AVP;multicast;destination=224.2.0.1;port=3456-3457;ttl=16").unwrap();
    assert_eq!(t.cast_mode, CastMode::Multicast);
    assert_eq!(t.destination, Some("224.2.0.1".to_string()));
    assert_eq!(t.port, Some((3456, 3457)));
    assert_eq!(t.ttl, Some(16));
}

#[test]
fn test_transport_parse_ssrc() {
    let t = Transport::parse("RTP/AVP;unicast;client_port=4588-4589;ssrc=DEADBEEF").unwrap();
    assert_eq!(t.ssrc, Some(0xDEADBEEF));
}

#[test]
fn test_transport_parse_record_mode() {
    let t = Transport::parse("RTP/AVP/TCP;unicast;interleaved=0-1;mode=\"RECORD\"").unwrap();
    assert_eq!(t.mode, TransportMode::Record);
}

#[test]
fn test_transport_serialize_tcp() {
    let t = Transport::tcp_interleaved(2, 3);
    let value = t.to_header_value();
    assert!(value.contains("RTP/AVP/TCP"));
    assert!(value.contains("unicast"));
    assert!(value.contains("interleaved=2-3"));
}

#[test]
fn test_transport_serialize_udp() {
    let t = Transport::udp_unicast(5000, 5001);
    let value = t.to_header_value();
    assert!(value.contains("RTP/AVP"));
    assert!(value.contains("unicast"));
    assert!(value.contains("client_port=5000-5001"));
}

#[test]
fn test_transport_serialize_record() {
    let t = Transport::tcp_interleaved(0, 1).with_mode_record();
    let value = t.to_header_value();
    assert!(value.contains("mode=\"RECORD\""));
}

// =============================================================================
// Round-Trip Tests
// =============================================================================

#[test]
fn test_request_roundtrip() {
    let req = Request::setup_with_transport(
        "rtsp://example.com/movie/trackID=1",
        "RTP/AVP/TCP;unicast;interleaved=0-1",
    )
    .with_cseq(3)
    .with_user_agent("vibeav/1.0");

    let bytes = req.to_bytes();
    let (parsed, _) = Request::parse(&bytes).unwrap();

    assert_eq!(parsed.method, req.method);
    assert_eq!(parsed.uri, req.uri);
    assert_eq!(parsed.cseq(), req.cseq());
}

#[test]
fn test_response_roundtrip() {
    let sdp = b"v=0\r\no=- 1234 1234 IN IP4 127.0.0.1\r\ns=Test\r\n";
    let resp = Response::ok()
        .with_cseq(2)
        .with_session_timeout("12345678", 60)
        .with_sdp(sdp.to_vec());

    let bytes = resp.to_bytes();
    let (parsed, _) = Response::parse(&bytes).unwrap();

    assert_eq!(parsed.status, resp.status);
    assert_eq!(parsed.cseq(), resp.cseq());
    assert_eq!(parsed.body, resp.body);
}

#[test]
fn test_transport_roundtrip() {
    let t = Transport::parse("RTP/AVP;unicast;client_port=4588-4589;server_port=6256-6257").unwrap();
    let value = t.to_header_value();
    let parsed = Transport::parse(&value).unwrap();

    assert_eq!(parsed.client_port, t.client_port);
    assert_eq!(parsed.server_port, t.server_port);
    assert_eq!(parsed.cast_mode, t.cast_mode);
}
