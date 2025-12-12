# Sections 5.2-5.3: Multiplexing RTP Sessions & Header Extensions

> [Back to Index](00-index.md) | [Previous: RTP Header](04-rtp-header.md) | [Next: RTCP Overview](06-rtcp-overview.md)

## 5.2 Multiplexing RTP Sessions

### Separate Sessions per Medium

**Recommendation**: Use separate RTP sessions for different media types.

```
Audio: UDP ports 5004/5005 (RTP/RTCP)
Video: UDP ports 5006/5007 (RTP/RTCP)
```

### Why Separate Sessions?

| Reason | Explanation |
|--------|-------------|
| Selective reception | Receiver can choose audio-only |
| Different QoS | Network can prioritize differently |
| Different processing | Separate decode pipelines |
| Firewall filtering | Block video, allow audio |

### Linking Sessions

Sessions from same participant linked by **CNAME**:

```
Audio RTCP SDES: CNAME=alice@example.com, SSRC=0x12345678
Video RTCP SDES: CNAME=alice@example.com, SSRC=0xABCDEF01
```

Receiver knows both streams belong to Alice.

---

## Session Multiplexing Options

### By Port (Recommended)

```
Session 1: 192.168.1.100:5004 (RTP), :5005 (RTCP)
Session 2: 192.168.1.100:5006 (RTP), :5007 (RTCP)
```

### By Multicast Address

```
Session 1: 224.1.1.1:5004
Session 2: 224.1.1.2:5004
```

### By SSRC (NOT Recommended)

Don't multiplex different media by SSRC:
- Breaks separate SSRC space assumption
- Complicates RTCP processing
- Violates "same timing space" property

---

## Payload Type Changes

Sender MAY change payload type mid-session.

**Valid use**:
```
Time 0-10s: PT=0 (PCMU)
Time 10s+:  PT=8 (PCMA)
```

**Invalid use**: Don't use PT for media multiplexing.

---

## 5.3 Profile-Specific Modifications

Profiles may modify the RTP header:

| Modification | Example |
|--------------|---------|
| Redefine marker bit | Different meaning per codec |
| Additional markers | Steal bits from PT field |
| Mandatory extensions | Require header extension |

### Example: AVPF Profile (RFC 4585)

Adds feedback capability, same header format.

---

## 5.3.1 RTP Header Extension

Optional extension follows fixed header (when X=1).

### Extension Format

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|      defined by profile       |           length              |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                        header extension                       |
|                             ....                              |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### Extension Fields

| Field | Bits | Description |
|-------|------|-------------|
| Defined by profile | 16 | Profile-specific identifier |
| Length | 16 | Extension length in 32-bit words (excluding this header) |
| Extension data | Variable | Profile-specific data |

**Length = 0 is valid** (4-byte extension header only).

### Position in Packet

```
+----------------+
| RTP Header     | 12 bytes
+----------------+
| CSRC List      | 0-60 bytes (if CC > 0)
+----------------+
| Extension Hdr  | 4 bytes (if X=1)
+----------------+
| Extension Data | length × 4 bytes
+----------------+
| Payload        | Variable
+----------------+
| Padding        | Variable (if P=1)
+----------------+
```

---

## Common Extensions

### One-Byte Header Extension (RFC 5285)

```
Profile ID: 0xBEDE

 0                   1
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|  ID   |  len  |    data...    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

| Field | Bits | Description |
|-------|------|-------------|
| ID | 4 | Extension element ID (1-14) |
| len | 4 | Data length - 1 (0 = 1 byte) |
| data | (len+1) × 8 | Extension data |

### Two-Byte Header Extension (RFC 5285)

```
Profile ID: 0x100X (X = appbits)

 0                   1                   2
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|      ID       |     len       |    data...    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

---

## Extension Use Cases

| Extension | Purpose | RFC |
|-----------|---------|-----|
| abs-send-time | Absolute send timestamp | - |
| toffset | Transmission time offset | RFC 5450 |
| audio-level | Audio level indication | RFC 6464 |
| video-orientation | Camera rotation | RFC 6184 |
| mid | Media identification | RFC 8843 |
| rid | Restriction identifier | RFC 8851 |

### Example: Audio Level (RFC 6464)

```
ID: (negotiated in SDP)
Len: 0 (1 byte of data)
Data: V + level (0-127 dBov)

 0 1 2 3 4 5 6 7
+-+-+-+-+-+-+-+-+
|V|   level     |
+-+-+-+-+-+-+-+-+
V = voice activity
```

---

## Parsing Header Extension

```c
int parse_rtp_extension(uint8_t *buf, size_t len,
                        uint16_t *profile, uint8_t **ext_data, size_t *ext_len) {
    if (len < 4) return -1;

    *profile = (buf[0] << 8) | buf[1];
    uint16_t words = (buf[2] << 8) | buf[3];
    *ext_len = words * 4;

    if (len < 4 + *ext_len) return -1;

    *ext_data = buf + 4;

    return 4 + *ext_len;  // Total extension size
}

// One-byte extension parsing
void parse_one_byte_extensions(uint8_t *data, size_t len) {
    size_t offset = 0;
    while (offset < len) {
        uint8_t byte = data[offset];
        if (byte == 0) {
            offset++;  // Padding
            continue;
        }

        uint8_t id = (byte >> 4) & 0x0F;
        uint8_t elem_len = (byte & 0x0F) + 1;

        if (id == 15) break;  // Terminator

        // Process extension element
        process_extension(id, data + offset + 1, elem_len);

        offset += 1 + elem_len;
    }
}
```

---

## When to Use Extensions

### Use Header Extension For:

- Timestamp adjustments
- Audio levels
- Video orientation
- Metadata that applies per-packet

### Don't Use Extension For:

- Payload-specific parameters → Use payload section
- Profile modifications → Use profile-specific header changes
- Large data → Use payload or RTCP

---

## Summary

| Topic | Recommendation |
|-------|----------------|
| Media multiplexing | Separate RTP sessions per medium |
| Session identification | Linked by CNAME in RTCP |
| Payload type | May change, don't use for mux |
| Header extension | Profile-specific, use sparingly |
| Extension format | One-byte (RFC 5285) preferred |

---

> [Back to Index](00-index.md) | [Previous: RTP Header](04-rtp-header.md) | [Next: RTCP Overview](06-rtcp-overview.md)
