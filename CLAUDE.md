# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
# Build entire workspace
cargo build

# Build specific crate
cargo build -p vibeav-rtsp

# Run tests for entire workspace
cargo test

# Run tests for specific crate
cargo test -p vibeav-core

# Run a single test
cargo test -p vibeav-core test_name

# Check without building
cargo check

# Release build (includes LTO, single codegen unit, symbol stripping)
cargo build --release
```

## Architecture

VibeAV is a **zero-copy, protocol-agnostic audio/video routing server**. It forwards media payloads as opaque byte slices without decoding or transcoding.

### Core Design

- **vibeav-core**: Central routing engine with sharded trie router, zero-copy payload references, per-connection output queues, and transport adapter traits
- **Root crate** (`src/main.rs`): Runtime initialization, configuration loading, transport registration, and main event loop
- **Protocol crates** (`crates/vibeav-*`): Each transport protocol (RTSP, RTP, SRT, WebRTC, etc.) lives in its own crate

### Key Constraints

- Media payloads are raw byte slices - never decode H.264, H.265, AAC, Opus, VP8, etc.
- No FFmpeg or GStreamer dependencies
- Avoid allocating new buffers per packet - prefer zero-copy
- Use `workspace = true` for shared dependency versions

### Protocol Specs

Detailed protocol specifications live in `docs/spec/{protocol}/` as ordered markdown files (00-index.md, 01-introduction.md, etc.). Use these for implementing protocol parsers and state machines.

**IMPORTANT**: When implementing protocols:
- **Only read the markdown files** in `docs/spec/`, NOT the raw RFC files (they are huge and will waste context)
- Implement all **normative aspects** (MUST, SHALL, REQUIRED) from each spec
- The markdown files are already broken down into AI-digestible chunks

### Code Organization

- **Keep files small** for context preservation
- **Put tests in separate files** if the main file is large (e.g., `rtp.rs` → tests in `tests/rtp_tests.rs`)
- Use `#[cfg(test)] mod tests;` with external test modules when needed
