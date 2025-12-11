# Section 5.1: RTP Fixed Header Fields

> [Back to Index](00-index.md) | [Previous: Byte Order](03-byte-order.md) | [Next: Multiplexing](05-multiplexing.md)

## RTP Header Format

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|V=2|P|X|  CC   |M|     PT      |       sequence number         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                           timestamp                           |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|           synchronization source (SSRC) identifier            |
+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
|            contributing source (CSRC) identifiers             |
|                             ....                              |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**Minimum size**: 12 bytes (without CSRC list)
**Maximum size**: 12 + (15 × 4) = 72 bytes (with 15 CSRCs)

---

## Field Descriptions

### Version (V): 2 bits

RTP version number.

| Value | Meaning |
|-------|---------|
| 0 | vat audio tool (obsolete) |
| 1 | First draft (obsolete) |
| **2** | **Current version** |
| 3 | Reserved |

**Always 2 for RFC 3550.**

---

### Padding (P): 1 bit

Indicates padding at end of payload.

| Value | Meaning |
|-------|---------|
| 0 | No padding |
| 1 | Padding present |

**When set**: Last byte of packet = count of padding bytes (including itself).

```
+----------------------------------+--------+
|          Payload data            | 0x04   |  ← 4 bytes of padding
+----------------------------------+--------+
                              ↑ padding count
```

**Use cases**:
- Encryption with fixed block sizes
- Lower-layer protocol requirements

---

### Extension (X): 1 bit

Indicates header extension present.

| Value | Meaning |
|-------|---------|
| 0 | No extension |
| 1 | Extension follows fixed header |

See [Multiplexing](05-multiplexing.md#header-extension) for extension format.

---

### CSRC Count (CC): 4 bits

Number of CSRC identifiers following the fixed header.

| Value | Meaning |
|-------|---------|
| 0 | No CSRCs (direct source) |
| 1-15 | Number of CSRCs (from mixer) |

---

### Marker (M): 1 bit

Profile-defined significant event marker.

**Common uses**:

| Profile/Codec | Marker Meaning |
|---------------|----------------|
| Audio | Start of talkspurt |
| Video (RFC 2435) | End of frame |
| Video (RFC 6184 H.264) | End of access unit |

**Purpose**: Allows receiver to optimize playout.

---

### Payload Type (PT): 7 bits

Identifies payload format.

| Range | Assignment |
|-------|------------|
| 0-34 | Static (RFC 3551) |
| 35-71 | Reserved |
| 72-76 | Reserved (RTCP conflict) |
| 77-95 | Reserved |
| **96-127** | **Dynamic** (SDP negotiated) |

**Common static types** (RFC 3551):

| PT | Encoding | Clock Rate | Channels |
|----|----------|------------|----------|
| 0 | PCMU (G.711 μ-law) | 8000 | 1 |
| 3 | GSM | 8000 | 1 |
| 8 | PCMA (G.711 A-law) | 8000 | 1 |
| 9 | G722 | 8000 | 1 |
| 14 | MPA (MPEG Audio) | 90000 | - |
| 26 | JPEG | 90000 | - |
| 31 | H261 | 90000 | - |
| 32 | MPV (MPEG Video) | 90000 | - |
| 33 | MP2T (MPEG-2 TS) | 90000 | - |

**Dynamic types** defined in SDP:
```
a=rtpmap:96 H264/90000
a=rtpmap:97 opus/48000/2
```

**Rules**:
- Receiver MUST ignore unknown payload types
- Sender MAY change PT during session

---

### Sequence Number: 16 bits

Increments by 1 for each RTP packet sent.

**Properties**:
- Initial value: random (security)
- Wraps from 65535 to 0
- Per-SSRC

**Uses**:
- Detect packet loss
- Restore packet order
- Count packets received

```
Packets: seq=100, seq=101, seq=103, seq=104
                          ↑ seq=102 lost
```

---

### Timestamp: 32 bits

Sampling instant of first octet in payload.

**Properties**:
- Clock rate depends on codec
- Initial value: random (security)
- Wraps from 2³² - 1 to 0
- Per-SSRC, monotonically increasing

**Clock rates**:

| Media | Clock Rate | Tick Duration |
|-------|------------|---------------|
| G.711 (8kHz) | 8000 | 125 μs |
| Opus (48kHz) | 48000 | 20.8 μs |
| Video (90kHz) | 90000 | 11.1 μs |

**Same timestamp** for packets of same frame:
```
Video frame split into 3 packets:
  Packet 1: ts=90000, seq=100, M=0
  Packet 2: ts=90000, seq=101, M=0
  Packet 3: ts=90000, seq=102, M=1  ← end of frame
```

**Timestamp increment** (audio example):
```
20ms audio at 8kHz:
  Packet 1: ts=0
  Packet 2: ts=160    (8000 * 0.020)
  Packet 3: ts=320
  ...
```

---

### SSRC: 32 bits

Synchronization source identifier.

**Properties**:
- Randomly generated
- Unique within RTP session
- Identifies packet stream

**Generation**: See [SSRC Allocation](11-ssrc.md).

**Collision**: If duplicate detected, must choose new SSRC.

---

### CSRC List: 0-15 × 32 bits

Contributing source identifiers (from mixer).

**Present only if CC > 0.**

```
Mixer combines sources A, B, C:

Output packet:
  SSRC = 0x12345678  (mixer's SSRC)
  CC = 3
  CSRC[0] = 0xAABBCCDD  (source A)
  CSRC[1] = 0x11223344  (source B)
  CSRC[2] = 0x55667788  (source C)
```

---

## Header Structure (C)

```c
typedef struct {
#if __BYTE_ORDER == __BIG_ENDIAN
    uint8_t version:2;
    uint8_t padding:1;
    uint8_t extension:1;
    uint8_t csrc_count:4;
    uint8_t marker:1;
    uint8_t payload_type:7;
#else
    uint8_t csrc_count:4;
    uint8_t extension:1;
    uint8_t padding:1;
    uint8_t version:2;
    uint8_t payload_type:7;
    uint8_t marker:1;
#endif
    uint16_t sequence;
    uint32_t timestamp;
    uint32_t ssrc;
    uint32_t csrc[0];  // Variable length
} rtp_header_t;
```

---

## Parsing Example

```c
int parse_rtp_header(uint8_t *buf, size_t len, rtp_header_t *hdr) {
    if (len < 12) return -1;  // Minimum header size

    uint8_t byte0 = buf[0];
    uint8_t byte1 = buf[1];

    hdr->version = (byte0 >> 6) & 0x03;
    hdr->padding = (byte0 >> 5) & 0x01;
    hdr->extension = (byte0 >> 4) & 0x01;
    hdr->csrc_count = byte0 & 0x0F;

    hdr->marker = (byte1 >> 7) & 0x01;
    hdr->payload_type = byte1 & 0x7F;

    hdr->sequence = (buf[2] << 8) | buf[3];
    hdr->timestamp = (buf[4] << 24) | (buf[5] << 16) | (buf[6] << 8) | buf[7];
    hdr->ssrc = (buf[8] << 24) | (buf[9] << 16) | (buf[10] << 8) | buf[11];

    // Validate
    if (hdr->version != 2) return -1;

    size_t header_size = 12 + (hdr->csrc_count * 4);
    if (len < header_size) return -1;

    return header_size;
}
```

---

## Summary

| Field | Bits | Offset | Description |
|-------|------|--------|-------------|
| V | 2 | 0 | Version (2) |
| P | 1 | 2 | Padding |
| X | 1 | 3 | Extension |
| CC | 4 | 4-7 | CSRC count |
| M | 1 | 8 | Marker |
| PT | 7 | 9-15 | Payload type |
| Seq | 16 | 16-31 | Sequence number |
| TS | 32 | 32-63 | Timestamp |
| SSRC | 32 | 64-95 | Source ID |
| CSRC | 32×CC | 96+ | Contributors |

---

> [Back to Index](00-index.md) | [Previous: Byte Order](03-byte-order.md) | [Next: Multiplexing](05-multiplexing.md)
