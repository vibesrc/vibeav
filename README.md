# VibeAV

VibeAV is a high-performance, zero-copy, protocol-agnostic audio/video routing server. It focuses on transport-layer media passthrough (RTP, RTSP, SRT, WebRTC, HLS, etc.) with minimal buffering, predictable latency, and a sharded, lock-free routing core.

VibeAV is not a transcoder, pipeline, or codec framework. Media payloads are treated as opaque byte slices and forwarded without decoding or modification.

## Core Principles

* Zero-copy packet forwarding whenever possible.
* No codec parsing or transcoding in the hot path.
* Modular protocol adapters (RTSP, RTP, SRT, WebRTC, etc.).
* Sharded trie router for scalable fanout.
* Async Rust with Tokio for deterministic I/O.
* Workspace organized as many small crates for clean separation.
* Root package (`vibeav/`) is the main executable.

## Workspace Structure (Simplified)

```
crates/
  vibeav-core        # routing engine (sharded trie, queues, sessions)
  vibeav-rtp         # RTP/RTCP transport parsing (RFC 3550 / 3551)
  vibeav-sdp         # SDP session description parser (RFC 4566)
  vibeav-rtsp        # RTSP protocol (client/server, RFC 2326)
  vibeav-srt         # SRT ingest/output transport
  vibeav-webrtc      # WHIP/WHEP and SRTP passthrough
  vibeav-hls         # HLS playlist/segment writer (optional)
  vibeav-rtmp        # RTMP ingest/output (optional)
  vibeav-mpegts      # MPEG-TS parser/writer (for TS-based ingest/egress)
  vibeav-quic        # WebTransport/QUIC-based transport (future)
  vibeav-*           # Additional transport crates (stubs for now)

vibeav/              # root executable
docs/                # protocol specifications (RTP/RTSP/SDP/etc.) and architecture notes
```

crates/
vibeav-core        # routing engine (sharded trie, queues, sessions)
vibeav-rtp
vibeav-sdp         # SDP parser (required for RTSP)         # RTP parsing and framing
vibeav-rtsp        # RTSP protocol (client/server)
vibeav-srt         # SRT ingest/output
vibeav-webrtc      # WHIP/WHEP and SRTP passthrough
vibeav-hls         # HLS playlist/segment writer (optional)
vibeav-rtmp        # RTMP ingest/output (optional)
vibeav-mpegts      # MPEG-TS parser/writer
vibeav-quic        # WebTransport/QUIC-based transport (future)
vibeav-*           # Additional transport crates (stubs for now)

vibeav/              # root executable
docs/                # protocol specs, architecture, design notes

```

## AI Development Guidance

- Place *transport-specific logic* (parsers, protocols, servers, clients) inside their respective crates under `crates/`.

- The `vibeav-core` crate contains:
  - the global router
  - sharded subscription/state logic
  - zero-copy payload references
  - per-connection output queues
  - transport adapter traits

- The root crate (`vibeav/src/main.rs`) should:
  - initialize the runtime
  - load configuration
  - register transports (RTSP, SRT, WebRTC, etc.)
  - start services
  - run the main event loop

- Media payloads should always be forwarded as raw byte slices or references. Do not decode H.264, H.265, AAC, Opus, VP8, etc.

- For crate-to-crate dependencies, prefer using `workspace = true` to share versions consistently.

## What Not To Do

- Do not write transcoding logic.
- Do not depend on FFmpeg or GStreamer.
- Do not manipulate codec bitstreams unless explicitly required for a format.
- Do not allocate new buffers per packet unless zero-copy is impossible.

## Documentation Structure

All protocol specifications live under `docs/spec/{protocol}`.
These documents are split into ordered Markdown files (e.g. `00-index.md`, `01-introduction.md`, etc.) to allow AI agents to ingest and implement protocol logic directly from the spec.

Example:
```

docs/spec/rtsp/

Also add:

* `docs/spec/sdp/` for SDP grammar and RFC 4566
  00-index.md
  01-introduction.md
  02-notation.md
  ...
  D-minimal-implementation.md

```

Other protocols follow the same pattern:
- `docs/spec/rtp/`
- `docs/spec/srt/`
- `docs/spec/webrtc/`
- `docs/spec/hls/`
- `docs/spec/rtmp/`
- `docs/spec/mpegts/`
- `docs/spec/llhls/`
- `docs/spec/sip/`
- `docs/spec/dvb/`
- `docs/spec/atsc/`
- `docs/spec/zixi/`

Each protocol folder is intended for:
- AI-driven code generation
- Parser/state-machine creation
- Conformance test generation
- Human-readable spec breakdown

---

## Future Work (AI may generate stubs but not full implementations yet)

- QUIC/WebTransport relay
- LL-HLS low-latency playlist generation
- WHIP/WHEP refinements
- RTMP ingest pipeline
- Recording/archiving modules (MP4, TS)

```
