# SDP Introduction

## Purpose

SDP is intended for describing multimedia sessions for:
- Session announcement (multicast)
- Session invitation (SIP)
- Session initiation (RTSP)
- Email and WWW distribution

SDP is purely a format for session description - it does not incorporate a transport protocol. It is designed to be used with different transport protocols including:
- Session Announcement Protocol (SAP)
- Session Initiation Protocol (SIP)
- Real Time Streaming Protocol (RTSP)
- Email (MIME)
- HTTP

## Terminology

| Term | Definition |
|------|------------|
| Conference | A set of two or more communicating users along with their software |
| Session | A set of multimedia senders/receivers and the data streams between them |
| Session Description | A well-defined format for conveying information to participate in a session |

## Requirements

An SDP session description conveys:

**REQUIRED information:**
- Session name and purpose
- Time(s) the session is active
- The media comprising the session
- Information to receive those media (addresses, ports, formats)

**OPTIONAL information:**
- Bandwidth requirements
- Contact information for responsible person

## General Format

SDP is entirely textual using UTF-8 encoding. Each line has the form:

```
<type>=<value>
```

Where:
- `<type>` is exactly one case-significant character
- `<value>` is structured text (format depends on type)
- No whitespace allowed on either side of `=`

## Session Structure

An SDP description consists of:
1. **Session-level section** - starts with `v=`, continues until first `m=`
2. **Media-level sections** - each starts with `m=`, continues until next `m=` or end

Session-level values are defaults for all media unless overridden.

### Required Order

Lines MUST appear in this order (enhances error detection):

```
Session description:
    v=  (protocol version)
    o=  (originator and session identifier)
    s=  (session name)
    i=* (session information)
    u=* (URI of description)
    e=* (email address)
    p=* (phone number)
    c=* (connection information)
    b=* (bandwidth information)
    One or more time descriptions
    z=* (time zone adjustments)
    k=* (encryption key)
    a=* (session attributes)
    Zero or more media descriptions

Time description:
    t=  (time the session is active)
    r=* (repeat times)

Media description:
    m=  (media name and transport address)
    i=* (media title)
    c=* (connection information)
    b=* (bandwidth information)
    k=* (encryption key)
    a=* (media attributes)
```

Items marked with `*` are OPTIONAL.

## Character Set

- Field names and attribute names: US-ASCII subset of UTF-8
- Text fields and attribute values: Full ISO 10646 (UTF-8)
- Lines terminated by CRLF (parsers SHOULD accept LF only)

**Prohibited bytes in text:**
- 0x00 (NUL)
- 0x0A (LF)
- 0x0D (CR)

## Parser Requirements

Per RFC 4566:

1. **Unknown type letters**: Parser MUST completely ignore the session description
2. **Unknown attributes**: Parser MUST ignore them
3. **Strict ordering**: Parser MAY reject out-of-order fields

## Example

```
v=0
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5
s=SDP Seminar
i=A Seminar on the session description protocol
u=http://www.example.com/seminars/sdp.pdf
e=j.doe@example.com (Jane Doe)
c=IN IP4 224.2.17.12/127
t=2873397496 2873404696
a=recvonly
m=audio 49170 RTP/AVP 0
m=video 51372 RTP/AVP 99
a=rtpmap:99 h263-1998/90000
```

This describes:
- Session version 0 (always)
- Originated by `jdoe` from `10.47.16.5`
- Named "SDP Seminar"
- Multicast to `224.2.17.12` with TTL 127
- Active during the specified NTP time range
- Receive-only session (applies to all media)
- Audio using PCM u-law (payload type 0)
- Video using H.263-1998 (dynamic payload type 99)
