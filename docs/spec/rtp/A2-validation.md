# Appendix B: Packet Validation

> [Back to Index](00-index.md) | [Previous: Algorithms](A1-algorithms.md)

## RTP Packet Validation

### Minimum Checks

```c
typedef enum {
    RTP_VALID,
    RTP_INVALID_VERSION,
    RTP_INVALID_LENGTH,
    RTP_INVALID_PAYLOAD_TYPE,
    RTP_INVALID_EXTENSION,
    RTP_INVALID_PADDING
} rtp_validation_result_t;

rtp_validation_result_t validate_rtp(uint8_t *buf, size_t len) {
    // Minimum RTP packet: 12 bytes (fixed header only)
    if (len < 12) {
        return RTP_INVALID_LENGTH;
    }

    // Version MUST be 2
    uint8_t version = (buf[0] >> 6) & 0x03;
    if (version != 2) {
        return RTP_INVALID_VERSION;
    }

    // Calculate minimum valid length
    uint8_t cc = buf[0] & 0x0F;
    size_t min_len = 12 + cc * 4;

    // Check extension
    uint8_t x = (buf[0] >> 4) & 0x01;
    if (x) {
        if (len < min_len + 4) {
            return RTP_INVALID_EXTENSION;
        }

        uint8_t *ext = buf + min_len;
        uint16_t ext_len = (ext[2] << 8) | ext[3];
        min_len += 4 + ext_len * 4;
    }

    // Check padding
    uint8_t p = (buf[0] >> 5) & 0x01;
    if (p) {
        if (len < min_len + 1) {
            return RTP_INVALID_PADDING;
        }

        uint8_t pad_len = buf[len - 1];
        if (pad_len == 0 || pad_len > len - min_len) {
            return RTP_INVALID_PADDING;
        }
    }

    // Check total length
    if (len < min_len) {
        return RTP_INVALID_LENGTH;
    }

    // Payload type checks (profile-specific)
    uint8_t pt = buf[1] & 0x7F;
    // PT 72-76 reserved to distinguish from RTCP
    if (pt >= 72 && pt <= 76) {
        return RTP_INVALID_PAYLOAD_TYPE;
    }

    return RTP_VALID;
}
```

---

## RTCP Compound Packet Validation

### Complete Validation

```c
typedef enum {
    RTCP_VALID,
    RTCP_INVALID_VERSION,
    RTCP_INVALID_LENGTH,
    RTCP_INVALID_FIRST_PACKET,
    RTCP_INVALID_PADDING,
    RTCP_INVALID_PACKET_TYPE
} rtcp_validation_result_t;

rtcp_validation_result_t validate_rtcp_compound(uint8_t *buf, size_t len) {
    // Minimum: empty RR (8 bytes)
    if (len < 8) {
        return RTCP_INVALID_LENGTH;
    }

    // Must be 32-bit aligned
    if (len % 4 != 0) {
        return RTCP_INVALID_LENGTH;
    }

    uint8_t *end = buf + len;
    uint8_t *p = buf;
    bool first = true;

    while (p < end) {
        if (p + 4 > end) {
            return RTCP_INVALID_LENGTH;
        }

        // Version MUST be 2
        uint8_t version = (p[0] >> 6) & 0x03;
        if (version != 2) {
            return RTCP_INVALID_VERSION;
        }

        uint8_t pt = p[1];

        // First packet MUST be SR (200) or RR (201)
        if (first) {
            if (pt != 200 && pt != 201) {
                return RTCP_INVALID_FIRST_PACKET;
            }
            first = false;
        }

        // Valid packet types: 200-204
        if (pt < 200 || pt > 204) {
            // Could be extended type - allow if > 204
            // For strict validation, uncomment:
            // return RTCP_INVALID_PACKET_TYPE;
        }

        // Get length (in 32-bit words, minus 1)
        uint16_t length = ((p[2] << 8) | p[3]) + 1;
        size_t packet_len = length * 4;

        if (p + packet_len > end) {
            return RTCP_INVALID_LENGTH;
        }

        // Check padding on last packet only
        uint8_t padding = (p[0] >> 5) & 0x01;
        if (padding) {
            if (p + packet_len != end) {
                // Padding set but not last packet
                return RTCP_INVALID_PADDING;
            }

            uint8_t pad_count = p[packet_len - 1];
            if (pad_count == 0 || pad_count > packet_len - 4) {
                return RTCP_INVALID_PADDING;
            }
        }

        p += packet_len;
    }

    // Must end exactly at boundary
    if (p != end) {
        return RTCP_INVALID_LENGTH;
    }

    return RTCP_VALID;
}
```

---

## Individual RTCP Packet Validation

### SR Packet

```c
bool validate_sr(uint8_t *buf, size_t len) {
    if (len < 28) return false;  // Header (8) + sender info (20)

    uint8_t rc = buf[0] & 0x1F;
    size_t expected = 28 + rc * 24;  // Each report block is 24 bytes

    // Account for padding
    uint8_t padding = (buf[0] >> 5) & 0x01;
    if (padding) {
        uint8_t pad_count = buf[len - 1];
        expected += pad_count;
    }

    return (len >= expected);
}
```

### RR Packet

```c
bool validate_rr(uint8_t *buf, size_t len) {
    if (len < 8) return false;  // Header only

    uint8_t rc = buf[0] & 0x1F;
    size_t expected = 8 + rc * 24;  // Each report block is 24 bytes

    // Account for padding
    uint8_t padding = (buf[0] >> 5) & 0x01;
    if (padding) {
        uint8_t pad_count = buf[len - 1];
        expected += pad_count;
    }

    return (len >= expected);
}
```

### SDES Packet

```c
bool validate_sdes(uint8_t *buf, size_t len) {
    if (len < 4) return false;

    uint8_t sc = buf[0] & 0x1F;
    uint8_t *p = buf + 4;
    uint8_t *end = buf + len;

    for (int i = 0; i < sc; i++) {
        // SSRC/CSRC
        if (p + 4 > end) return false;
        p += 4;

        // Items
        while (p < end && *p != 0) {
            if (p + 2 > end) return false;
            uint8_t item_len = p[1];
            if (p + 2 + item_len > end) return false;
            p += 2 + item_len;
        }

        // Skip END and padding
        if (p >= end) return false;
        p++;  // END item
        while ((p - buf) % 4 != 0 && p < end) {
            p++;  // Padding
        }
    }

    return true;
}
```

### BYE Packet

```c
bool validate_bye(uint8_t *buf, size_t len) {
    if (len < 4) return false;

    uint8_t sc = buf[0] & 0x1F;
    size_t min_len = 4 + sc * 4;

    if (len < min_len) return false;

    // Optional reason string
    uint8_t *p = buf + 4 + sc * 4;
    if (p < buf + len) {
        uint8_t reason_len = *p;
        min_len += 1 + reason_len;
        // Padding to 32-bit boundary
        while (min_len % 4 != 0) min_len++;
    }

    return (len >= min_len);
}
```

### APP Packet

```c
bool validate_app(uint8_t *buf, size_t len) {
    // Minimum: header (4) + SSRC (4) + name (4) = 12 bytes
    if (len < 12) return false;

    // Length field indicates total - check consistency
    uint16_t length = ((buf[2] << 8) | buf[3]) + 1;
    return (len == length * 4);
}
```

---

## Distinguishing RTP from RTCP

### Port-Based (Traditional)

```
RTP:  Even port (e.g., 5004)
RTCP: Odd port (e.g., 5005)
```

### Multiplexed (RFC 5761)

When RTP and RTCP share a port:

```c
bool is_rtcp_packet(uint8_t *buf, size_t len) {
    if (len < 2) return false;

    // RTCP payload types: 200-204 (and extended: 205-211)
    uint8_t pt = buf[1];

    // RTP PT 72-76 are reserved to avoid conflict
    // RTCP PT 200-204 are standard
    return (pt >= 200 && pt <= 211);
}

void handle_packet(uint8_t *buf, size_t len, int port) {
    if (is_rtcp_packet(buf, len)) {
        process_rtcp(buf, len);
    } else {
        process_rtp(buf, len);
    }
}
```

---

## Source Validation

### Validating New Sources

```c
#define MIN_SEQUENTIAL 2

typedef struct {
    uint32_t ssrc;
    bool validated;
    int sequential_count;
    uint16_t expected_seq;
} source_validator_t;

bool validate_source(source_validator_t *v, uint16_t seq) {
    if (v->validated) {
        return true;
    }

    if (v->sequential_count == 0) {
        // First packet
        v->expected_seq = seq + 1;
        v->sequential_count = 1;
        return false;
    }

    if (seq == v->expected_seq) {
        v->sequential_count++;
        v->expected_seq = seq + 1;

        if (v->sequential_count >= MIN_SEQUENTIAL) {
            v->validated = true;
            return true;
        }
    } else {
        // Reset validation
        v->sequential_count = 1;
        v->expected_seq = seq + 1;
    }

    return false;
}
```

---

## Summary

| Check | Purpose | Required |
|-------|---------|----------|
| Version | Must be 2 | Yes |
| Length | Packet size valid | Yes |
| First RTCP | SR or RR | Yes |
| Padding | Valid count | Yes |
| PT 72-76 | Reserved range | Recommended |
| Source validation | Probation period | Recommended |

---

> [Back to Index](00-index.md) | [Previous: Algorithms](A1-algorithms.md)
