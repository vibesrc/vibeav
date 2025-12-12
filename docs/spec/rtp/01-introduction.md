# Section 1: Introduction

> [Back to Index](00-index.md) | [Next: Definitions](02-definitions.md)

## Purpose

RTP provides end-to-end delivery services for data with real-time characteristics:
- Interactive audio and video
- Simulation data
- Any data requiring timing preservation

### Core Services

| Service | Description |
|---------|-------------|
| Payload identification | What codec/format is being used |
| Sequence numbering | Detect loss, restore order |
| Timestamping | Synchronization, jitter calculation |
| Delivery monitoring | Quality feedback via RTCP |

### What RTP Does NOT Provide

| Not Provided | Why |
|--------------|-----|
| Timely delivery | Relies on lower layers |
| QoS guarantees | No reservation mechanism |
| Reliable delivery | UDP-based, packets can be lost |
| In-order delivery | Sequence numbers for receiver to reorder |

---

## Design Philosophy

RTP follows **Application Level Framing (ALF)**:

1. **Malleable protocol** - Tailored per application
2. **Integrated processing** - Often part of application, not separate layer
3. **Deliberately incomplete** - Requires profile + payload format specs

### Protocol Components

```
+------------------+
|   Application    |
+------------------+
|       RTP        | ← This spec (RFC 3550)
+------------------+
|      Profile     | ← e.g., RFC 3551 (Audio/Video)
+------------------+
| Payload Format   | ← e.g., RFC 6184 (H.264)
+------------------+
|       UDP        |
+------------------+
|       IP         |
+------------------+
```

---

## Two Parts

### 1. RTP (Data Protocol)

Carries media data with:
- Fixed 12-byte header (minimum)
- Optional CSRC list
- Optional header extension
- Payload data

### 2. RTCP (Control Protocol)

Provides:
- **Quality feedback** - Reception reports
- **Participant identification** - CNAME
- **Synchronization** - NTP/RTP timestamp correlation
- **Session control** - BYE packets

RTCP uses separate port (typically RTP port + 1).

---

## Use Scenarios

### 2.1 Simple Multicast Audio Conference

```
Participant A                    Multicast Group
     |                               |
     |-------- RTP Audio ----------->|
     |                               |
     |<------- RTP Audio ------------|  (from B, C, D...)
     |                               |
     |-------- RTCP Reports -------->|
     |<------- RTCP Reports ---------|
```

- Each participant sends audio to multicast group
- RTCP reports provide quality feedback
- BYE packet sent when leaving

### 2.2 Audio and Video Conference

```
Audio RTP Session (port 5004/5005)
Video RTP Session (port 5006/5007)
```

- **Separate RTP sessions** for each medium
- Same CNAME links participant's audio and video
- RTCP SR timestamps enable lip-sync

### 2.3 Mixers and Translators

#### Mixer

```
Source A ─┐
Source B ─┼──> [MIXER] ──> Mixed Output
Source C ─┘
              (new SSRC,
               CSRC list)
```

- Combines multiple sources
- Generates new timing
- Inserts CSRC list identifying contributors

#### Translator

```
Source A ──> [TRANSLATOR] ──> Source A (modified)
                              (same SSRC)
```

- Preserves SSRC
- May change encoding, bridge networks
- Examples: transcoder, firewall relay

### 2.4 Layered Encodings

```
Base Layer ────> Multicast Group 1 (required)
Enhancement 1 ──> Multicast Group 2 (optional)
Enhancement 2 ──> Multicast Group 3 (optional)
```

- Separate RTP session per layer
- Receivers subscribe to layers they can handle
- Enables heterogeneous receivers

---

## Companion Documents

### Required

| Document | Purpose |
|----------|---------|
| Profile | Maps payload types to formats |
| Payload Format | Defines how to carry specific codec |

### Common Profiles

| RFC | Profile |
|-----|---------|
| RFC 3551 | RTP/AVP - Audio/Video Profile |
| RFC 4585 | RTP/AVPF - Extended feedback |
| RFC 3711 | SRTP - Secure RTP |

### Common Payload Formats

| RFC | Format |
|-----|--------|
| RFC 6184 | H.264 Video |
| RFC 7798 | H.265/HEVC Video |
| RFC 3640 | MPEG-4 Audio (AAC) |
| RFC 6716 | Opus Audio |

---

## Terminology

See [Definitions](02-definitions.md) for full terminology.

Key terms:
- **SSRC** - Synchronization Source (unique per sender)
- **CSRC** - Contributing Source (in mixed streams)
- **CNAME** - Canonical Name (persistent identifier)

---

## Relationship to Other Protocols

### RTSP

[RTSP](../rtsp/00-index.md) controls RTP sessions:
- SETUP establishes RTP transport
- PLAY starts RTP delivery
- RTP-Info header provides initial seq/timestamp

See [RTSP RTP Interaction](../rtsp/B-rtp-interaction.md).

### SDP

SDP describes RTP sessions:
- Media type and port
- Payload type mappings
- SSRC hints

See [RTSP SDP Usage](../rtsp/C-sdp-usage.md).

### UDP

RTP typically runs over UDP:
- RTP on even port (e.g., 5004)
- RTCP on next odd port (e.g., 5005)

---

> [Back to Index](00-index.md) | [Next: Definitions](02-definitions.md)
