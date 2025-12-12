# RFC 3550 & 3551: RTP - A Transport Protocol for Real-Time Applications

> **Source**: RFC 3550 (July 2003), RFC 3551 (July 2003)
> **Status**: Standards Track (Obsoletes RFC 1889, 1890)
> **Authors**: H. Schulzrinne, S. Casner, R. Frederick, V. Jacobson

## Overview

RTP provides end-to-end network transport functions for real-time data (audio, video, simulation). It consists of two closely-linked parts:

1. **RTP** - Carries data with real-time properties
2. **RTCP** - Monitors quality of service and conveys participant info

**Key characteristics:**
- Payload type identification
- Sequence numbering
- Timestamping
- Delivery monitoring
- Typically runs over UDP

**Related protocols:**
- [RTSP](../rtsp/00-index.md) - Controls RTP sessions
- SDP - Describes RTP sessions

---

## Document Index

### Core Protocol (RFC 3550)

| Section | File | Description |
|---------|------|-------------|
| 1 | [01-introduction.md](01-introduction.md) | Purpose, use scenarios, design philosophy |
| 2-3 | [02-definitions.md](02-definitions.md) | Terminology and definitions |
| 4 | [03-byte-order.md](03-byte-order.md) | Byte order, alignment, NTP time format |
| 5 | [04-rtp-header.md](04-rtp-header.md) | RTP fixed header fields |
| 5.2-5.3 | [05-multiplexing.md](05-multiplexing.md) | Session multiplexing, header extensions |

### RTCP Control Protocol (RFC 3550)

| Section | File | Description |
|---------|------|-------------|
| 6.1-6.3 | [06-rtcp-overview.md](06-rtcp-overview.md) | RTCP format and transmission rules |
| 6.4 | [07-rtcp-reports.md](07-rtcp-reports.md) | Sender/Receiver Reports (SR/RR) |
| 6.5 | [08-rtcp-sdes.md](08-rtcp-sdes.md) | Source Description (SDES) packets |
| 6.6-6.7 | [09-rtcp-bye-app.md](09-rtcp-bye-app.md) | BYE and APP packets |

### Advanced Topics (RFC 3550)

| Section | File | Description |
|---------|------|-------------|
| 7 | [10-translators-mixers.md](10-translators-mixers.md) | RTP translators and mixers |
| 8 | [11-ssrc.md](11-ssrc.md) | SSRC allocation, collision detection |
| 9-10 | [12-security.md](12-security.md) | Security and congestion control |

### Audio/Video Profile (RFC 3551)

| Section | File | Description |
|---------|------|-------------|
| All | [13-avp-profile.md](13-avp-profile.md) | RTP/AVP profile, payload types, codecs |

### Appendices

| Appendix | File | Description |
|----------|------|-------------|
| A.1-A.8 | [A1-algorithms.md](A1-algorithms.md) | Sequence handling, loss, jitter, RTCP timing |
| A.2 | [A2-validation.md](A2-validation.md) | RTP/RTCP packet validation |

---

## Quick Reference

### RTP Header (12 bytes minimum)

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|V=2|P|X|  CC   |M|     PT      |       sequence number         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                           timestamp                           |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|           synchronization source (SSRC) identifier            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

| Field | Bits | Description |
|-------|------|-------------|
| V | 2 | Version (2) |
| P | 1 | Padding flag |
| X | 1 | Extension flag |
| CC | 4 | CSRC count |
| M | 1 | Marker bit |
| PT | 7 | Payload type |
| Seq | 16 | Sequence number |
| Timestamp | 32 | Media timestamp |
| SSRC | 32 | Synchronization source ID |

### RTCP Packet Types

| Type | Value | Description |
|------|-------|-------------|
| SR | 200 | Sender Report |
| RR | 201 | Receiver Report |
| SDES | 202 | Source Description |
| BYE | 203 | Goodbye |
| APP | 204 | Application-defined |

### Static Audio Payload Types (RFC 3551)

| PT | Encoding | Clock (Hz) | Channels |
|----|----------|------------|----------|
| 0 | PCMU | 8,000 | 1 |
| 3 | GSM | 8,000 | 1 |
| 4 | G723 | 8,000 | 1 |
| 8 | PCMA | 8,000 | 1 |
| 9 | G722 | 8,000 | 1 |
| 10 | L16 | 44,100 | 2 |
| 11 | L16 | 44,100 | 1 |
| 14 | MPA | 90,000 | - |
| 18 | G729 | 8,000 | 1 |

### Static Video Payload Types (RFC 3551)

| PT | Encoding | Clock (Hz) |
|----|----------|------------|
| 26 | JPEG | 90,000 |
| 31 | H261 | 90,000 |
| 32 | MPV | 90,000 |
| 33 | MP2T | 90,000 |
| 34 | H263 | 90,000 |

### Dynamic Payload Types

| Range | Usage |
|-------|-------|
| 96-127 | Dynamic (signaled via SDP) |

Common dynamic types:
- Opus, VP8, VP9, H.264, H.265, etc.

---

## Implementation Notes

For implementing RTP/RTCP, read these files in order:

1. [01-introduction.md](01-introduction.md) - Understand design philosophy
2. [02-definitions.md](02-definitions.md) - Key terminology (SSRC, CSRC, etc.)
3. [04-rtp-header.md](04-rtp-header.md) - Packet format
4. [07-rtcp-reports.md](07-rtcp-reports.md) - SR/RR for synchronization
5. [11-ssrc.md](11-ssrc.md) - Collision handling
6. [A1-algorithms.md](A1-algorithms.md) - Reference implementations

For RTSP integration, see:
- [RTSP RTP Interaction](../rtsp/B-rtp-interaction.md)
- [RTSP Transport Header](../rtsp/10-transport-header.md)

---

## Port Assignments

| Protocol | Default Port |
|----------|--------------|
| RTP | 5004 (even) |
| RTCP | 5005 (odd) |

RTP/RTCP port pairs are always consecutive (RTP on even, RTCP = RTP + 1).
