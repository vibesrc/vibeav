# Sections 6.6-6.7: BYE and APP Packets

> [Back to Index](00-index.md) | [Previous: RTCP SDES](08-rtcp-sdes.md) | [Next: Translators/Mixers](10-translators-mixers.md)

## 6.6 BYE: Goodbye Packet

Indicates participant is leaving the session.

### Packet Format

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|V=2|P|    SC   |   PT=BYE=203  |             length            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                           SSRC/CSRC                           |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
:                              ...                              :
+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
|     length    |               reason for leaving            ...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### Fields

| Field | Description |
|-------|-------------|
| SC | Number of SSRCs (1-31) |
| SSRC/CSRC | SSRCs being terminated |
| length | Reason string length (optional) |
| reason | UTF-8 text explanation (optional) |

### Usage

```
When leaving session:
1. Send BYE as last RTCP packet
2. Stop sending RTP/RTCP
3. May include reason string
```

### Examples

**Simple BYE (no reason)**:
```
V=2, P=0, SC=1, PT=203, length=1
SSRC = 0x12345678
```

**BYE with reason**:
```
V=2, P=0, SC=1, PT=203, length=4
SSRC = 0x12345678
length = 11
reason = "Going away"
padding = 0x00 (to 32-bit boundary)
```

### Mixer BYE

Mixer sends BYE listing multiple CSRCs when sources leave:

```
SC = 3
SSRC[0] = 0x11111111
SSRC[1] = 0x22222222
SSRC[2] = 0x33333333
```

---

## BYE Transmission Rules

### When to Send

| Situation | Action |
|-----------|--------|
| Leaving session | Send BYE |
| SSRC collision | Send BYE for old SSRC |
| Application exit | Send BYE |

### Large Sessions

For sessions > 50 members:
- Use scaled BYE transmission
- Avoids BYE flood when session ends
- See [Algorithms](A3-algorithms.md) for algorithm

### Processing Received BYE

```c
void process_bye(uint32_t *ssrcs, int count, char *reason) {
    for (int i = 0; i < count; i++) {
        rtp_source_t *src = find_source(ssrcs[i]);
        if (src) {
            src->validated = false;
            remove_from_member_list(src);
            log_info("Source %08x left: %s", ssrcs[i], reason ? reason : "");
        }
    }
    reconsider_rtcp_interval();
}
```

---

## 6.7 APP: Application-Defined Packet

Custom RTCP packet for application-specific use.

### Packet Format

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|V=2|P| subtype |   PT=APP=204  |             length            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                           SSRC/CSRC                           |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                          name (ASCII)                         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                   application-dependent data                ...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### Fields

| Field | Bits | Description |
|-------|------|-------------|
| subtype | 5 | Application-defined subtype |
| SSRC | 32 | Source identifier |
| name | 32 | 4-character ASCII identifier |
| data | Variable | Application data |

### Name Field

4-character ASCII string identifying application:

```
Examples:
  "VLC " - VLC media player
  "ZOOM" - Zoom
  "OPAL" - Open Phone Abstraction Library
```

Choose unique name to avoid conflicts.

---

## APP Packet Uses

| Use Case | Description |
|----------|-------------|
| Feedback | Application-specific feedback |
| Commands | Control messages |
| Metadata | Additional stream info |
| Testing | Experimental features |

### Example: Custom Feedback

```c
// Application-defined "quality request" message
struct app_quality_req {
    uint8_t version;     // subtype in header
    uint8_t pt;          // 204
    uint16_t length;
    uint32_t ssrc;
    char name[4];        // "QUAL"
    uint8_t quality_level;  // 0-100
    uint8_t reserved[3];
};
```

---

## Implementation

### Generating BYE

```c
int generate_bye(uint8_t *buf, uint32_t ssrc, const char *reason) {
    uint8_t *p = buf;

    // Header
    *p++ = 0x81;  // V=2, SC=1
    *p++ = 203;   // PT=BYE

    size_t reason_len = reason ? strlen(reason) : 0;
    size_t total_len = 4;  // SSRC

    if (reason_len > 0) {
        total_len += 1 + reason_len;  // length byte + reason
        // Pad to 32-bit boundary
        while ((total_len + 4) % 4 != 0) total_len++;
    }

    uint16_t length = total_len / 4;
    *p++ = length >> 8;
    *p++ = length & 0xFF;

    // SSRC
    write_uint32(p, ssrc);
    p += 4;

    // Reason (optional)
    if (reason_len > 0) {
        *p++ = reason_len;
        memcpy(p, reason, reason_len);
        p += reason_len;
        // Padding
        while ((p - buf) % 4 != 0) *p++ = 0;
    }

    return p - buf;
}
```

### Generating APP

```c
int generate_app(uint8_t *buf, uint32_t ssrc, uint8_t subtype,
                 const char *name, const uint8_t *data, size_t data_len) {
    uint8_t *p = buf;

    // Header
    *p++ = 0x80 | (subtype & 0x1F);  // V=2, subtype
    *p++ = 204;  // PT=APP

    uint16_t length = 2 + (data_len + 3) / 4;  // SSRC + name + data
    *p++ = length >> 8;
    *p++ = length & 0xFF;

    // SSRC
    write_uint32(p, ssrc);
    p += 4;

    // Name (4 chars, pad with spaces)
    for (int i = 0; i < 4; i++) {
        *p++ = i < strlen(name) ? name[i] : ' ';
    }

    // Data
    memcpy(p, data, data_len);
    p += data_len;

    // Padding
    while ((p - buf) % 4 != 0) *p++ = 0;

    return p - buf;
}
```

### Parsing BYE

```c
void parse_bye(uint8_t *buf, size_t len) {
    uint8_t sc = buf[0] & 0x1F;
    uint8_t *p = buf + 4;

    uint32_t ssrcs[31];
    for (int i = 0; i < sc && i < 31; i++) {
        ssrcs[i] = read_uint32(p);
        p += 4;
    }

    char *reason = NULL;
    if (p < buf + len) {
        uint8_t reason_len = *p++;
        if (reason_len > 0 && p + reason_len <= buf + len) {
            reason = malloc(reason_len + 1);
            memcpy(reason, p, reason_len);
            reason[reason_len] = '\0';
        }
    }

    process_bye(ssrcs, sc, reason);
    free(reason);
}
```

---

## Summary

### BYE Packet

| Aspect | Detail |
|--------|--------|
| PT | 203 |
| Purpose | Leave notification |
| Contains | 1-31 SSRCs + optional reason |
| When | Last packet before leaving |

### APP Packet

| Aspect | Detail |
|--------|--------|
| PT | 204 |
| Purpose | Application-specific |
| Contains | Name (4 chars) + data |
| When | As needed by application |

---

> [Back to Index](00-index.md) | [Previous: RTCP SDES](08-rtcp-sdes.md) | [Next: Translators/Mixers](10-translators-mixers.md)
