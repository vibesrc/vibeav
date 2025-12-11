# RFC 4566: Session Description Protocol (SDP)

> **Source**: RFC 4566 (July 2006)
> **Status**: Standards Track
> **Authors**: M. Handley, V. Jacobson, C. Perkins
> **Obsoletes**: RFC 2327, RFC 3266

## Overview

SDP provides a standard format for describing multimedia sessions for session announcement, invitation, and initiation. It is purely a format for session description - it does not incorporate a transport protocol.

**Key characteristics:**
- Text-based format using UTF-8 encoding
- Line-oriented with `<type>=<value>` syntax
- Session-level and media-level sections
- Used with RTSP, SIP, SAP, and other protocols
- Media type: `application/sdp`

## Document Index

### Core Protocol

| Section | File | Description |
|---------|------|-------------|
| 1-4 | [01-introduction.md](01-introduction.md) | Purpose, terminology, requirements |
| 5.1-5.6 | [02-session-description.md](02-session-description.md) | Session-level fields (v, o, s, i, u, e, p) |
| 5.7-5.12 | [03-connection-timing.md](03-connection-timing.md) | Connection, bandwidth, timing, encryption |
| 5.13-5.14 | [04-attributes-media.md](04-attributes-media.md) | Attributes and media descriptions |
| 6 | [05-standard-attributes.md](05-standard-attributes.md) | Standard attribute definitions |
| 9 | [06-grammar.md](06-grammar.md) | ABNF syntax definitions |

### Reference

| Section | File | Description |
|---------|------|-------------|
| 7 | [07-security.md](07-security.md) | Security considerations |
| 8 | [08-iana.md](08-iana.md) | IANA registrations and procedures |

---

## Quick Reference

### Session Description Structure

```
v=  (protocol version, always 0)
o=  (originator and session identifier)
s=  (session name)
i=* (session information)
u=* (URI of description)
e=* (email address)
p=* (phone number)
c=* (connection information)
b=* (bandwidth information)
t=  (time the session is active)
r=* (repeat times)
z=* (time zone adjustments)
k=* (encryption key - deprecated)
a=* (session attributes)
m=  (media description)
```

### Media Description Structure

```
m=  (media name and transport address)
i=* (media title)
c=* (connection information)
b=* (bandwidth information)
k=* (encryption key)
a=* (media attributes)
```

### Field Types

| Type | Field | Required | Description |
|------|-------|----------|-------------|
| `v=` | Version | Yes | Protocol version (always 0) |
| `o=` | Origin | Yes | Session originator |
| `s=` | Session Name | Yes | Textual session name |
| `i=` | Information | No | Session/media description |
| `u=` | URI | No | Pointer to more information |
| `e=` | Email | No | Contact email |
| `p=` | Phone | No | Contact phone |
| `c=` | Connection | Yes* | Network connection info |
| `b=` | Bandwidth | No | Proposed bandwidth |
| `t=` | Timing | Yes | Session active times |
| `r=` | Repeat | No | Repeat schedule |
| `z=` | Time Zone | No | DST adjustments |
| `k=` | Key | No | Encryption key (deprecated) |
| `a=` | Attribute | No | Extension mechanism |
| `m=` | Media | No | Media description |

*Required at session or media level

### Common Attributes

| Attribute | Level | Description |
|-----------|-------|-------------|
| `rtpmap` | Media | RTP payload type mapping |
| `fmtp` | Media | Format-specific parameters |
| `control` | Either | RTSP control URL |
| `range` | Session | Presentation time range |
| `recvonly` | Either | Receive-only mode |
| `sendrecv` | Either | Send and receive mode |
| `sendonly` | Either | Send-only mode |
| `inactive` | Either | Inactive mode |

### Example SDP

```
v=0
o=- 1234567890 1234567890 IN IP4 192.168.1.1
s=Example Session
i=A streaming media session
c=IN IP4 0.0.0.0
t=0 0
a=control:rtsp://example.com/stream
a=range:npt=0-
m=video 0 RTP/AVP 96
a=rtpmap:96 H264/90000
a=fmtp:96 profile-level-id=42001f; packetization-mode=1
a=control:trackID=1
m=audio 0 RTP/AVP 97
a=rtpmap:97 mpeg4-generic/48000/2
a=fmtp:97 streamtype=5; profile-level-id=1; mode=AAC-hbr
a=control:trackID=2
```

---

## Related Protocols

| Protocol | Documentation | Description |
|----------|---------------|-------------|
| RTSP | [RTSP Specification](../rtsp/00-index.md) | Streaming control |
| RTP | [RTP Specification](../rtp/00-index.md) | Media transport |
| SIP | External | Session initiation |

---

## Implementation Notes

For implementing an SDP parser, read these files in order:
1. [01-introduction.md](01-introduction.md) - Understand the format
2. [02-session-description.md](02-session-description.md) - Session fields
3. [04-attributes-media.md](04-attributes-media.md) - Media descriptions
4. [05-standard-attributes.md](05-standard-attributes.md) - rtpmap, fmtp, etc.
5. [06-grammar.md](06-grammar.md) - Formal syntax

For RTSP integration:
- See [RTSP SDP Usage](../rtsp/C-sdp-usage.md) for how RTSP uses SDP
- The `a=control:` attribute provides track URLs
- Port 0 in media lines means use RTSP SETUP for transport
