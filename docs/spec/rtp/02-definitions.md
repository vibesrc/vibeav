# Sections 2-3: Definitions

> [Back to Index](00-index.md) | [Previous: Introduction](01-introduction.md) | [Next: Byte Order](03-byte-order.md)

## Core Terminology

### Packet Types

| Term | Definition |
|------|------------|
| **RTP payload** | The data transported by RTP (audio samples, video frames, etc.) |
| **RTP packet** | Fixed header + optional CSRC list + payload |
| **RTCP packet** | Control packet with fixed header + structured elements |
| **Compound RTCP packet** | Multiple RTCP packets concatenated, sent as one UDP packet |

### Addressing

| Term | Definition |
|------|------------|
| **Port** | Transport-layer endpoint identifier (e.g., UDP port) |
| **Transport address** | Network address + port (e.g., IP + UDP port) |

---

## Session Concepts

### RTP Session

An association among participants communicating with RTP.

**Characteristics:**
- Identified by transport address pair (RTP + RTCP ports)
- Separate SSRC space per session
- One medium per session (typically)

```
RTP Session = {
  RTP transport address,
  RTCP transport address,
  Set of SSRCs,
  Payload type mapping
}
```

### Multimedia Session

A set of concurrent RTP sessions among common participants.

```
Multimedia Session
├── Audio RTP Session (ports 5004/5005)
├── Video RTP Session (ports 5006/5007)
└── (linked by common CNAME)
```

### RTP Media Type

Collection of payload types that can be carried in a single RTP session.

---

## Source Identifiers

### SSRC (Synchronization Source)

32-bit identifier for a stream of RTP packets.

**Characteristics:**
- Randomly chosen
- Unique within RTP session
- May change (collision, restart)
- All packets from same SSRC share timing/sequence space

**Examples of SSRC sources:**
- Microphone → audio stream
- Camera → video stream
- Mixer → combined stream

```
Packet from Camera:
  SSRC = 0xA1B2C3D4
  Seq = 1000
  Timestamp = 90000
```

### CSRC (Contributing Source)

SSRCs that contributed to a mixed packet.

**When used:**
- Mixer combines multiple inputs
- Original SSRCs listed as CSRCs
- Enables talker identification

```
Mixer output packet:
  SSRC = 0x12345678  (mixer's SSRC)
  CC = 3             (3 contributors)
  CSRC[0] = 0xA1B2C3D4  (source 1)
  CSRC[1] = 0xE5F6A7B8  (source 2)
  CSRC[2] = 0x98765432  (source 3)
```

### CNAME (Canonical Name)

Persistent identifier for a participant.

**Characteristics:**
- Transported in RTCP SDES packets
- Survives SSRC changes
- Links multiple RTP sessions (audio + video)
- Format: `user@host` or unique string

```
CNAME = "alice@example.com"
```

See [RTCP SDES](08-rtcp-sdes.md) for CNAME format details.

---

## Participant Types

### End System

Application that generates/consumes RTP packets.

```
[Microphone] → [Encoder] → [RTP Sender] → Network
Network → [RTP Receiver] → [Decoder] → [Speaker]
```

### Mixer

Combines multiple RTP sources into one output.

**Behavior:**
- Receives from multiple sources
- Resynchronizes timing
- Generates new SSRC
- Lists original SSRCs as CSRCs

```
Source A (SSRC=A) ─┐
                   ├──> [MIXER] ──> Output (SSRC=M, CSRC=[A,B,C])
Source B (SSRC=B) ─┤
                   │
Source C (SSRC=C) ─┘
```

**Use cases:**
- Audio conferencing (mix voices)
- Video compositing
- Bandwidth reduction for low-speed links

### Translator

Forwards RTP packets with SSRC intact.

**Behavior:**
- Preserves original SSRC
- May modify encoding
- Doesn't mix sources

```
Source (SSRC=A) ──> [TRANSLATOR] ──> Output (SSRC=A, maybe different encoding)
```

**Use cases:**
- Transcoding (change codec)
- Multicast-to-unicast bridge
- Firewall traversal

### Monitor

Receives RTCP to observe session quality.

**Types:**
- Built into application
- Third-party (doesn't send/receive RTP)

---

## Non-RTP Means

Protocols/mechanisms needed alongside RTP:

| Function | Protocol Examples |
|----------|-------------------|
| Session setup | [RTSP](../rtsp/00-index.md), SIP, H.323 |
| Session description | SDP |
| Encryption key exchange | DTLS-SRTP, MIKEY |
| Payload type negotiation | SDP offer/answer |

---

## Identifier Relationships

```
Participant "Alice"
│
├── CNAME: alice@192.168.1.100
│
├── Audio RTP Session
│   └── SSRC: 0xA1B2C3D4
│       ├── Seq: 1000, 1001, 1002...
│       └── Timestamp: 160, 320, 480...
│
└── Video RTP Session
    └── SSRC: 0xE5F6A7B8
        ├── Seq: 5000, 5001, 5002...
        └── Timestamp: 3000, 6000, 9000...
```

**Key relationships:**
- CNAME links SSRCs from same participant
- SSRC identifies packet stream within session
- Seq/Timestamp are per-SSRC

---

## Timing Terms

| Term | Definition |
|------|------------|
| **RTP timestamp** | Media clock time of first sample in packet |
| **NTP timestamp** | Wallclock time (in RTCP SR) |
| **Sampling instant** | When media was captured |
| **Playout time** | When media should be rendered |

### Clock Relationships

```
Media Clock (per codec):
  Audio 8kHz:   1 tick = 125 μs
  Audio 48kHz:  1 tick = 20.8 μs
  Video 90kHz:  1 tick = 11.1 μs

NTP Clock:
  64-bit: seconds since 1900 + fractional seconds
  Resolution: ~200 picoseconds
```

See [Byte Order](03-byte-order.md) for NTP timestamp format.

---

## Quick Reference

| Identifier | Bits | Scope | Persistence |
|------------|------|-------|-------------|
| SSRC | 32 | Per session | May change |
| CSRC | 32 | Per packet | N/A |
| CNAME | Variable | Global | Persistent |
| Seq | 16 | Per SSRC | Wraps |
| Timestamp | 32 | Per SSRC | Wraps |
| PT | 7 | Per session | Fixed by profile |

---

> [Back to Index](00-index.md) | [Previous: Introduction](01-introduction.md) | [Next: Byte Order](03-byte-order.md)
