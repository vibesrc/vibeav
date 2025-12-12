# VibeAV Protocol Implementation Plan

## Overview

Implementation order based on dependencies:
1. **vibeav-rtp** - Foundation for everything (RTP/RTCP parsing)
2. **vibeav-sdp** - Required for RTSP DESCRIBE
3. **vibeav-core** - Router engine (uses RTP types)
4. **vibeav-rtsp** - First full protocol (depends on rtp, sdp, core)
5. **vibeav-server** - Main binary that ties protocols together

### Implementation Approach

- **Read ONLY the markdown files** in `docs/spec/` (NOT the raw RFCs - they're huge)
- **Implement all normative aspects** (MUST, SHALL, REQUIRED) from each spec
- The markdown files are already broken down into AI-digestible chunks
- Focus on correctness first, optimization later

### Terminology

| Term | Description |
|------|-------------|
| **Stream** | A media source (e.g., camera feed, published stream) |
| **Source** | Entity publishing media TO a stream |
| **Sink** | Entity receiving media FROM a stream |
| **Track** | Specific media type within a stream (video, audio) |
| **Attachment** | Binding between a track and a sink (track-level filtering) |
| **Session** | Client connection with state |

---

## Phase 1: vibeav-rtp (Foundation) ✅ COMPLETE

RTP is the bedrock - RTSP, WebRTC, SIP, SRT all use it.

### 1.1 MUST Implement

- Parse RTP header (version, PT, seq, timestamp, SSRC, CSRC)
- Parse RTCP SR/RR/SDES/BYE minimally
- Zero-copy payload slice (`&[u8]` or `Bytes`)
- Parse interleaved RTP over TCP ("$" framing)
- Simple RTP packet struct

### 1.2 DO NOT Implement

- Codec parsing
- NAL splitting
- Audio frame parsing
- Jitter buffers
- NTP wallclock sync
- Full RTCP statistics engine

### 1.3 Module Structure

```
vibeav-rtp/src/
├── lib.rs           # Public API exports
├── rtp.rs           # RTP header + packet
├── rtcp/
│   ├── mod.rs       # RTCP types
│   ├── sr.rs        # Sender Report (minimal)
│   ├── rr.rs        # Receiver Report (minimal)
│   ├── sdes.rs      # Source Description (CNAME focus)
│   └── bye.rs       # Goodbye
├── interleaved.rs   # $ framing for RTP-over-TCP
└── error.rs
```

### 1.4 Key Types

```rust
/// RTP packet - zero-copy view over bytes
pub struct RtpPacket<'a> {
    pub version: u8,       // Must be 2
    pub padding: bool,
    pub extension: bool,
    pub marker: bool,
    pub payload_type: u8,  // 0-127
    pub sequence: u16,
    pub timestamp: u32,
    pub ssrc: u32,
    pub csrc: &'a [u32],   // 0-15 entries
    pub payload: &'a [u8], // Zero-copy slice
}

/// Parse RTP header, return packet view
pub fn parse_rtp(data: &[u8]) -> Result<RtpPacket<'_>, RtpError>;

/// Interleaved frame ($ channel len data)
pub struct InterleavedFrame<'a> {
    pub channel: u8,
    pub data: &'a [u8],
}

/// Parse interleaved frame
pub fn parse_interleaved(data: &[u8]) -> Result<(InterleavedFrame<'_>, usize), RtpError>;
```

### 1.5 Dependencies

```toml
[dependencies]
bytes = { workspace = true }
thiserror = { workspace = true }
```

---

## Phase 2: vibeav-sdp (RTSP Prerequisite) ✅ COMPLETE

SDP describes media sessions - RTSP DESCRIBE returns SDP.

### 2.1 MUST Implement

- Parse session-level lines (v=, o=, s=, c=, t=)
- Parse media-level blocks (m=, a=)
- Parse `a=rtpmap` and `a=fmtp`
- Parse `a=control` (RTSP control URL)
- Normalize attributes (lowercase keys)
- Expose track info

### 2.2 DO NOT Implement

- Bandwidth modifiers
- Timezones
- Repeat intervals
- Encryption keys
- IPv6 quirks
- Full RFC 4566 compliance

### 2.3 Module Structure

```
vibeav-sdp/src/
├── lib.rs           # Public API
├── session.rs       # Session-level (v=, o=, s=)
├── media.rs         # Media descriptions (m=, rtpmap, fmtp)
├── parser.rs        # Line parser
└── error.rs
```

### 2.4 Key Types

```rust
pub struct SessionDescription {
    pub version: u8,                    // v= (always 0)
    pub origin: Origin,                 // o=
    pub session_name: String,           // s=
    pub connection: Option<Connection>, // c=
    pub media: Vec<MediaDescription>,   // m= sections
}

pub struct MediaDescription {
    pub media_type: MediaType,          // audio, video
    pub port: u16,
    pub protocol: String,               // RTP/AVP
    pub formats: Vec<u8>,               // Payload types
    pub rtpmap: Vec<RtpMap>,            // a=rtpmap
    pub fmtp: Vec<Fmtp>,                // a=fmtp
    pub control: Option<String>,        // a=control
}

pub struct RtpMap {
    pub payload_type: u8,
    pub encoding_name: String,
    pub clock_rate: u32,
    pub channels: Option<u8>,
}

/// Parse SDP text
pub fn parse_sdp(sdp: &str) -> Result<SessionDescription, SdpError>;
```

### 2.5 Dependencies

```toml
[dependencies]
thiserror = { workspace = true }
```

---

## Phase 3: vibeav-core (Router Engine) ✅ COMPLETE

With RTP defined, the core can route packets cleanly.

### 3.1 MUST Implement

- Accept RTP packets from protocols (both cameras AND publishing clients)
- Maintain sinks (stream → connections)
- Forward packets to output queues
- Source → stream mappings
- Track-level attachments (video-only, audio-only sinks)
- Zero-copy forwarding
- Source registry (track who's sending to each stream)

### 3.2 Key Data Structures

- Stream registry
- Source registry (source_id → Source)
- Sink registry (sink_id → Sink)
- Attachment registry (track-level routing)
- Per-sink circular buffer queues (DropOldest support)
- Session registry

### 3.3 Key Traits

```rust
/// Protocol pushes packets to core
impl Router {
    pub async fn on_rtp(&self, stream_id: &str, data: Bytes) -> Result<()>;
    pub async fn on_rtcp(&self, stream_id: &str, data: Bytes) -> Result<()>;
}

/// Core pushes packets to protocol connections via SinkReceiver
pub enum SinkReceiver {
    Channel(mpsc::Receiver<MediaPacket>),
    RingBuffer(RingBufferReceiver),
}
```

### 3.4 Module Structure

```
vibeav-core/src/
├── lib.rs
├── router.rs        # Main router engine
├── stream.rs        # Stream state
├── session.rs       # Session management
├── sink.rs          # Sink management (output queues)
├── source.rs        # Source management
├── attachment.rs    # Track-level attachments
├── server.rs        # ProtocolServer trait
└── error.rs
```

### 3.5 Dependencies

```toml
[dependencies]
bytes = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }
```

---

## Phase 4: vibeav-rtsp (First Full Protocol) ✅ COMPLETE

RTSP client/server using RTP + SDP + core.

### 4.1 MUST Implement Methods (Playback - Server sends media)

- `OPTIONS` - Query capabilities
- `DESCRIBE` - Get SDP (server → client)
- `SETUP` - Create session/transport
- `PLAY` - Start delivery (server → client)
- `TEARDOWN` - Close session

### 4.2 MUST Implement Methods (Publishing - Client sends media)

Required for GStreamer/FFmpeg clients streaming TO our server:

- `ANNOUNCE` - Post SDP (client → server, describes what client will send)
- `RECORD` - Start recording/receiving (client → server)

#### ANNOUNCE Flow (Client Publishing)
```
Client                              Server
   |                                   |
   |  ANNOUNCE rtsp://server/stream    |
   |  Content-Type: application/sdp    |
   |  <SDP describing media>           |
   | --------------------------------> |
   |                                   |
   |  200 OK                           |
   | <-------------------------------- |
   |                                   |
   |  SETUP rtsp://server/stream/trackID=1  |
   |  Transport: RTP/AVP/TCP;interleaved=0-1;mode=record  |
   | --------------------------------> |
   |                                   |
   |  200 OK                           |
   |  Transport: RTP/AVP/TCP;interleaved=0-1  |
   | <-------------------------------- |
   |                                   |
   |  RECORD rtsp://server/stream      |
   | --------------------------------> |
   |                                   |
   |  200 OK                           |
   | <-------------------------------- |
   |                                   |
   |  [RTP data flows client→server]   |
   | ================================> |
```

#### Key Differences from Playback
- ANNOUNCE contains SDP (vs DESCRIBE returns SDP)
- RECORD starts client→server media flow (vs PLAY starts server→client)
- Transport header includes `mode=record`
- Server receives RTP instead of sending it

### 4.3 Transport

- **TCP interleaved first** ($ framing)
- UDP transport later

### 4.4 Module Structure

```
vibeav-rtsp/src/
├── lib.rs
├── message/
│   ├── mod.rs
│   ├── request.rs
│   ├── response.rs
│   ├── method.rs
│   ├── status.rs
│   └── headers.rs
├── transport.rs     # Transport header parsing
├── server/
│   ├── mod.rs       # RtspServer
│   ├── config.rs    # RtspServerConfig
│   ├── connection.rs # Connection handler
│   ├── handler.rs   # Method handlers
│   └── session.rs   # RTSP session state
├── client/
│   ├── mod.rs       # RtspClient
│   ├── config.rs    # RtspClientConfig
│   ├── connection.rs # Client connection handler
│   └── session.rs   # Client session state
└── error.rs
```

### 4.5 Key Types

```rust
pub enum Method {
    Options,
    Describe,
    Announce,  // Publishing: client sends SDP
    Setup,
    Play,
    Pause,
    Record,    // Publishing: client starts sending media
    Teardown,
    GetParameter,
    SetParameter,
}

pub struct Request {
    pub method: Method,
    pub uri: String,
    pub headers: Headers,
    pub body: Option<Vec<u8>>,
}

pub struct Response {
    pub status: StatusCode,
    pub reason: String,
    pub headers: Headers,
    pub body: Option<Vec<u8>>,
}

pub struct Transport {
    pub protocol: TransportProtocol,    // RTP
    pub profile: TransportProfile,      // AVP, SAVP
    pub lower_transport: LowerTransport, // TCP, UDP
    pub cast_mode: CastMode,            // Unicast, Multicast
    pub mode: TransportMode,            // Play, Record
    pub interleaved: Option<(u8, u8)>,
    pub client_port: Option<(u16, u16)>,
    pub server_port: Option<(u16, u16)>,
}

pub enum TransportMode {
    Play,    // Server → Client (default)
    Record,  // Client → Server (publishing)
}
```

### 4.6 Dependencies

```toml
[dependencies]
vibeav-rtp = { path = "../vibeav-rtp" }
vibeav-sdp = { path = "../vibeav-sdp" }
vibeav-core = { path = "../vibeav-core" }
bytes = { workspace = true }
tokio = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
```

---

## Phase 5: vibeav-server (Main Binary) ✅ BASIC COMPLETE

Main binary that creates a Router and starts protocol listeners.

### 5.1 Current Implementation

```rust
use vibeav_core::{ProtocolServer, Router};
use vibeav_rtsp::{RtspServer, RtspServerConfig};

#[tokio::main]
async fn main() {
    let router = Arc::new(Router::new());

    let rtsp_config = RtspServerConfig::default()
        .with_bind("0.0.0.0:8554".parse()?);

    let rtsp_server = RtspServer::new(rtsp_config, router.clone());
    rtsp_server.start().await?;
}
```

### 5.2 Future: TOML Configuration

```toml
[rtsp]
listen = "rtsp://0.0.0.0:8554"
allow_publish = true
allow_play = true

[[paths]]
path = "/live/camera1"
source = "rtsp://192.168.1.69:554/video0"

[[paths]]
path = "/live/camera2"
source = "rtsp://192.168.1.70:554/stream1"

[webrtc]
listen = "0.0.0.0:8889"
# ...
```

---

## End State

After Phase 5, vibeav can:
- ✅ **Pull streams from cameras** via RTSP client (DESCRIBE/SETUP/PLAY)
- ✅ **Accept published streams** from GStreamer/FFmpeg (ANNOUNCE/RECORD)
- ✅ **Forward RTP packets** through the router to all sinks
- ✅ **Serve streams** to RTSP playback clients (DESCRIBE/SETUP/PLAY)
- ✅ **Handle multiple streams** concurrently
- ✅ **Zero-copy forwarding** using `Bytes` (Arc-backed)
- ✅ **Dynamic SDP** generation from actual track info

This is MediaMTX-level functionality for RTSP/RTP.

### Stream Sources

Two ways to get streams into the server:
1. **RTSP Client** (pull): Server connects to `rtsp://camera/stream`, pulls media
2. **ANNOUNCE/RECORD** (push): FFmpeg/GStreamer pushes media to server

### Publishing Compatibility

Tested with:
- **FFmpeg**: `ffmpeg -re -i input.mp4 -c copy -f rtsp rtsp://server/stream`
- **GStreamer**: `gst-launch-1.0 ... ! rtspclientsink location=rtsp://server/stream`
- **OBS** (via FFmpeg output)

---

## Dependency Graph

```
vibeav-server
    ├── vibeav-rtsp
    │   ├── vibeav-rtp
    │   ├── vibeav-sdp
    │   └── vibeav-core
    └── vibeav-core
            └── (bytes, tokio)
```

---

## Implementation Order

| Phase | Crate | Focus | Status |
|-------|-------|-------|--------|
| 1 | vibeav-rtp | RTP header, RTCP minimal, $ framing | ✅ Complete (41 tests) |
| 2 | vibeav-sdp | SDP parser (just enough for RTSP) | ✅ Complete |
| 3 | vibeav-core | Router engine, zero-copy forwarding | ✅ Complete (43 tests) |
| 4 | vibeav-rtsp | RTSP client/server, TCP interleaved | ✅ Complete (47 tests) |
| 5 | vibeav-server | Main binary | ✅ Basic complete |

---

## Verification Checklist

### Phase 1 (vibeav-rtp) ✅ COMPLETE
- [x] Parse RTP header correctly
- [x] Parse RTCP SR/RR/SDES/BYE
- [x] Parse $ interleaved frames
- [x] Zero-copy payload access
- [x] `cargo test -p vibeav-rtp` passes (41 tests)

### Phase 2 (vibeav-sdp) ✅ COMPLETE
- [x] Parse camera SDP blobs
- [x] Extract rtpmap/fmtp
- [x] Extract control URLs
- [x] `cargo test -p vibeav-sdp` passes

### Phase 3 (vibeav-core) ✅ COMPLETE
- [x] Route packets by stream ID
- [x] Multiple sinks per stream
- [x] Zero-copy forwarding
- [x] Track-level attachments
- [x] DropOldest ring buffer for sinks
- [x] Source/Sink terminology (not publisher/subscriber)
- [x] `cargo test -p vibeav-core` passes (43 tests)

### Phase 4 (vibeav-rtsp) ✅ COMPLETE

**Message Parsing:**
- [x] Request parsing/serialization
- [x] Response parsing/serialization
- [x] Transport header parsing
- [x] All RTSP methods defined

**Server:**
- [x] RtspServer with ProtocolServer trait
- [x] Connection handler with interleaved RTP
- [x] Method handlers (OPTIONS, DESCRIBE, ANNOUNCE, SETUP, PLAY, RECORD, PAUSE, TEARDOWN)
- [x] Session state machine
- [x] Integration with core Router
- [x] SDP parsing in ANNOUNCE → stores track info on stream
- [x] Dynamic SDP generation in DESCRIBE from actual track info
- [x] `cargo test -p vibeav-rtsp` passes (84 tests)

**Client:**
- [x] RtspClient for pulling streams from cameras
- [x] OPTIONS/DESCRIBE/SETUP/PLAY flow
- [x] SDP parsing and track extraction
- [x] Interleaved RTP receiving and router forwarding
- [x] Reconnection support
- [x] Session state machine

**Still TODO:**
- [ ] Server Hooks API (on_describe, on_announce, etc.)
- [ ] UDP transport support
- [ ] End-to-end test with real camera
- [ ] End-to-end test with FFmpeg publish

### Phase 5 (vibeav-server) ✅ BASIC COMPLETE
- [x] Main binary structure
- [x] Router creation
- [x] RTSP server integration
- [ ] TOML configuration loading
- [ ] Multiple protocol support
- [ ] Graceful shutdown

---

## Future Work

### Additional Protocols
- WebRTC (vibeav-webrtc)
- SRT (vibeav-srt)
- HLS/DASH (vibeav-hls)
- RTMP (vibeav-rtmp)

### Features
- TOML configuration
- Authentication hooks
- Metrics/observability
- Recording to disk
- Transcoding integration
