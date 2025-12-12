# Section 6.5: SDES - Source Description

> [Back to Index](00-index.md) | [Previous: RTCP Reports](07-rtcp-reports.md) | [Next: RTCP BYE/APP](09-rtcp-bye-app.md)

## SDES Purpose

Provides human-readable and machine-parseable identification for RTP sources.

**Key item**: CNAME links SSRCs across:
- SSRC changes (collision, restart)
- Multiple RTP sessions (audio + video)

---

## SDES Packet Format

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|V=2|P|    SC   |  PT=SDES=202  |             length            |
+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
|                          SSRC/CSRC_1                          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                           SDES items                          |
|                              ...                              |
+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
|                          SSRC/CSRC_2                          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                           SDES items                          |
|                              ...                              |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**SC**: Source count (number of SSRC/CSRC chunks)

---

## SDES Item Format

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|    Type       |     Length    |          value ...            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

| Field | Bytes | Description |
|-------|-------|-------------|
| Type | 1 | Item type code |
| Length | 1 | Value length (0-255) |
| Value | Variable | Item content (UTF-8) |

**End marker**: Type=0 (no length/value)

---

## SDES Item Types

| Type | Name | Required | Description |
|------|------|----------|-------------|
| 0 | END | - | End of item list |
| 1 | **CNAME** | **Yes** | Canonical name |
| 2 | NAME | No | User name |
| 3 | EMAIL | No | Email address |
| 4 | PHONE | No | Phone number |
| 5 | LOC | No | Location |
| 6 | TOOL | No | Application name |
| 7 | NOTE | No | Status message |
| 8 | PRIV | No | Private extension |

---

## 6.5.1 CNAME: Canonical End-Point Identifier

**Most important SDES item.**

### Format

```
user@host
```

### Requirements

| Requirement | Description |
|-------------|-------------|
| Unique | Globally unique within session |
| Persistent | Survives SSRC changes |
| Stable | Same across related sessions |
| Bound | Links audio/video from same user |

### CNAME Examples

```
alice@192.168.1.100
bob@laptop.example.com
conference-bridge@sip.example.org
random-id-12345@example.com
```

### Generation Rules

**With user identity**:
```
CNAME = user "@" host

user = login name or application ID
host = FQDN or IP address
```

**Without user identity**:
```
CNAME = random-hex "@" host

random-hex = random bytes, hex encoded
```

### Why CNAME Matters

```
Scenario: Alice restarts application

Before restart:
  Audio SSRC = 0x12345678

After restart (new random SSRC):
  Audio SSRC = 0xABCDEF01

Both identified by:
  CNAME = alice@192.168.1.100

Receiver knows it's still Alice.
```

---

## 6.5.2 NAME: User Name

Human-readable name.

```
Type: 2
Example: "Alice Smith"
```

### Notes
- For display in UI
- Not for identification (use CNAME)
- May be localized

---

## 6.5.3 EMAIL: Email Address

```
Type: 3
Example: "alice@example.com"
Format: RFC 2822 addr-spec
```

---

## 6.5.4 PHONE: Phone Number

```
Type: 4
Example: "+1 555 123 4567"
Format: ITU-T E.164 recommended
```

---

## 6.5.5 LOC: Geographic Location

```
Type: 5
Example: "Conference Room B, Building 3"
```

Application-dependent format.

---

## 6.5.6 TOOL: Application Name

```
Type: 6
Example: "VLC 3.0.16"
```

Identifies generating software.

---

## 6.5.7 NOTE: Status Message

```
Type: 7
Example: "In a meeting until 3pm"
```

Transient status, use sparingly (bandwidth).

---

## 6.5.8 PRIV: Private Extension

```
Type: 8
Format: prefix + value

 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|   Type=8      |     Length    | prefix length |   prefix ...  |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                        ... value ...                          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

Allows application-specific extensions.

---

## SDES Chunk Structure

Each SSRC/CSRC has a chunk:

```
+------------------------------------------+
|             SSRC/CSRC (32 bits)          |
+------------------------------------------+
| Type=1 | Len  | CNAME value ...          |  ← Required
+------------------------------------------+
| Type=2 | Len  | NAME value ...           |  ← Optional
+------------------------------------------+
| Type=0 | (padding to 32-bit boundary)    |  ← End marker
+------------------------------------------+
```

---

## Bandwidth Considerations

### CNAME: Every Report

Include CNAME in every SDES packet.

### Other Items: Round-Robin

Additional items sent less frequently:

```
Report 1: CNAME
Report 2: CNAME
Report 3: CNAME + NAME
Report 4: CNAME
Report 5: CNAME
Report 6: CNAME + EMAIL
...
```

Suggested allocation (from spec):
- CNAME: Every report
- NAME: Every 3rd report (7/8 of "extra" slots)
- EMAIL: Every 8th report (1/8 of "extra" slots)

---

## Implementation

### Generating SDES

```c
int generate_sdes(uint8_t *buf, uint32_t ssrc, const char *cname,
                  const char *name) {
    uint8_t *p = buf;

    // Header
    *p++ = 0x81;  // V=2, SC=1
    *p++ = 202;   // PT=SDES
    uint8_t *len_ptr = p;
    p += 2;  // Length (fill in later)

    // SSRC
    write_uint32(p, ssrc);
    p += 4;

    // CNAME item
    size_t cname_len = strlen(cname);
    *p++ = 1;  // CNAME
    *p++ = cname_len;
    memcpy(p, cname, cname_len);
    p += cname_len;

    // Optional NAME item
    if (name) {
        size_t name_len = strlen(name);
        *p++ = 2;  // NAME
        *p++ = name_len;
        memcpy(p, name, name_len);
        p += name_len;
    }

    // End marker and padding
    *p++ = 0;  // END
    while ((p - buf) % 4 != 0) {
        *p++ = 0;  // Padding
    }

    // Fill in length
    uint16_t length = ((p - buf) / 4) - 1;
    len_ptr[0] = length >> 8;
    len_ptr[1] = length & 0xFF;

    return p - buf;
}
```

### Parsing SDES

```c
void parse_sdes(uint8_t *buf, size_t len) {
    uint8_t sc = buf[0] & 0x1F;
    uint8_t *p = buf + 4;

    for (int i = 0; i < sc; i++) {
        uint32_t ssrc = read_uint32(p);
        p += 4;

        // Parse items
        while (*p != 0) {
            uint8_t type = *p++;
            uint8_t item_len = *p++;
            char value[256];
            memcpy(value, p, item_len);
            value[item_len] = '\0';
            p += item_len;

            process_sdes_item(ssrc, type, value);
        }

        // Skip END and padding
        p++;
        while ((p - buf) % 4 != 0) p++;
    }
}
```

---

## Summary

| Item | Type | Required | Purpose |
|------|------|----------|---------|
| CNAME | 1 | **Yes** | Unique identifier |
| NAME | 2 | No | Display name |
| EMAIL | 3 | No | Contact info |
| PHONE | 4 | No | Contact info |
| LOC | 5 | No | Location |
| TOOL | 6 | No | Software ID |
| NOTE | 7 | No | Status |
| PRIV | 8 | No | Extensions |

---

> [Back to Index](00-index.md) | [Previous: RTCP Reports](07-rtcp-reports.md) | [Next: RTCP BYE/APP](09-rtcp-bye-app.md)
