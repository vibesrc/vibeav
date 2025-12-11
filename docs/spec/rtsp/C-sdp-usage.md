# Appendix C: Use of SDP for RTSP Session Descriptions

> [Back to Index](00-index.md) | [Previous: RTP Interaction](B-rtp-interaction.md) | [Next: Minimal Implementation](D-minimal-implementation.md)

## Overview

SDP (Session Description Protocol, RFC 2327) describes RTSP presentations. SDP provides codec info, transport parameters, and control URLs.

**Related sections:**
- [DESCRIBE method](07-methods.md#102-describe) - How to request SDP
- [Protocol Parameters](03-protocol-parameters.md) - URL and time formats
- [RTP Interaction](B-rtp-interaction.md) - How SDP relates to RTP

---

## SDP in RTSP Context

### Usage Scenarios

| Scenario | Description |
|----------|-------------|
| DESCRIBE response | Server returns SDP for presentation |
| ANNOUNCE request | Client/server posts SDP description |
| Out-of-band | SDP retrieved via HTTP, email, etc. |

### Content-Type

```
Content-Type: application/sdp
```

---

## C.1 Definitions

### C.1.1 Control URL (`a=control:`)

Specifies URL for RTSP control.

#### Session-Level (Aggregate Control)

```
v=0
o=- 1234 1234 IN IP4 192.168.1.1
s=Presentation
a=control:rtsp://server.example.com/movie/
m=video 0 RTP/AVP 96
a=control:trackID=1
m=audio 0 RTP/AVP 97
a=control:trackID=2
```

#### Media-Level (Stream Control)

```
m=video 0 RTP/AVP 96
a=control:rtsp://server.example.com/movie/video
```

#### Relative URLs

Resolved against base URL:

```
Content-Base: rtsp://server.example.com/movie/

m=video 0 RTP/AVP 96
a=control:trackID=1
→ rtsp://server.example.com/movie/trackID=1
```

#### Base URL Resolution Order

1. RTSP `Content-Base` header
2. RTSP `Content-Location` header
3. RTSP Request URL (from DESCRIBE)

#### Asterisk Control URL

```
a=control:*
```

Means: Use the base URL (inherits from session/Content-Base).

---

### C.1.2 Media Streams (`m=`)

```
m=<media> <port> <proto> <fmt-list>
```

| Field | Description |
|-------|-------------|
| media | audio, video, text, application |
| port | Transport port (0 for RTSP) |
| proto | RTP/AVP typically |
| fmt-list | Payload type(s) |

#### Port Value

For unicast RTSP:
- Port should be **0** (client chooses in SETUP)
- Server port is a suggestion only

```
m=video 0 RTP/AVP 96
m=audio 0 RTP/AVP 97
```

For multicast, port may be specified:

```
m=video 3456 RTP/AVP 96
c=IN IP4 224.2.0.1/16
```

---

### C.1.3 Payload Types

#### Static Payload Types

```
m=audio 0 RTP/AVP 0     ; PCMU
m=audio 0 RTP/AVP 8     ; PCMA
m=video 0 RTP/AVP 26    ; JPEG
m=video 0 RTP/AVP 31    ; H.261
```

#### Dynamic Payload Types (96-127)

```
m=video 0 RTP/AVP 96
a=rtpmap:96 H264/90000
a=fmtp:96 profile-level-id=42e01f

m=audio 0 RTP/AVP 97
a=rtpmap:97 mpeg4-generic/44100/2
a=fmtp:97 mode=AAC-hbr; ...
```

---

### C.1.4 Format-Specific Parameters (`a=fmtp:`)

Codec-specific configuration:

```
a=fmtp:<payload-type> <parameters>
```

#### Examples

```
; H.264
a=fmtp:96 profile-level-id=42e01f; packetization-mode=1;
          sprop-parameter-sets=Z0IAH5WoFAFu,aM4G4g==

; AAC
a=fmtp:97 streamtype=5; profile-level-id=1; mode=AAC-hbr;
          sizelength=13; indexlength=3; indexdeltalength=3

; MPEG-4 Video
a=fmtp:98 profile-level-id=1; config=000001B0...
```

---

### C.1.5 Range of Presentation (`a=range:`)

Total duration of stored media:

```
a=range:npt=0-634.10           ; 634.10 seconds
a=range:npt=0-                 ; Unknown duration (live)
a=range:smpte=0:0:0-1:30:0     ; 1 hour 30 minutes
a=range:clock=19970113T2115-   ; Absolute time (live)
```

### C.1.6 Time of Availability (`t=`)

Session validity period:

```
t=0 0                          ; Always available
t=3034423619 3042462419        ; Specific time window
```

For aggregate control: Validity of description.
For non-aggregate: Actual availability.

---

### C.1.7 Connection Information (`c=`)

#### Unicast (Client Specifies)

```
c=IN IP4 0.0.0.0               ; Null address
```

Client provides destination in SETUP.

#### Multicast (Server Specifies)

```
c=IN IP4 224.2.0.1/16          ; Multicast group with TTL
```

---

### C.1.8 Entity Tag (`a=etag:`)

Version identifier for conditional SETUP:

```
a=etag:158bb3e7c7fd62ce67f12b533f06b83a
```

Used with `If-Match` header:

```
SETUP rtsp://server/movie/trackID=1 RTSP/1.0
If-Match: "158bb3e7c7fd62ce67f12b533f06b83a"
```

---

## C.2 Aggregate Control Not Available

Streams controlled independently (potentially different servers).

### Example

```
v=0
o=- 2890844256 2890842807 IN IP4 204.34.34.32
s=I came from a web page
t=0 0
c=IN IP4 0.0.0.0
m=video 8002 RTP/AVP 31
a=control:rtsp://video.com/movie.vid
m=audio 8004 RTP/AVP 3
a=control:rtsp://audio.com/movie.aud
```

**Note**: Different servers for video and audio. Client establishes separate RTSP sessions.

---

## C.3 Aggregate Control Available

Single server, unified control.

### Example

```
v=0
o=- 2890844256 2890842807 IN IP4 204.34.34.32
s=Presentation
i=A movie with audio and video
t=0 0
c=IN IP4 0.0.0.0
a=control:rtsp://example.com/movie/
m=video 0 RTP/AVP 96
a=rtpmap:96 H264/90000
a=control:trackID=1
m=audio 0 RTP/AVP 97
a=rtpmap:97 mpeg4-generic/44100/2
a=control:trackID=2
```

**URLs**:
- Aggregate: `rtsp://example.com/movie/`
- Video: `rtsp://example.com/movie/trackID=1`
- Audio: `rtsp://example.com/movie/trackID=2`

---

## Complete SDP Example

```
v=0
o=- 2890844256 2890842807 IN IP4 192.168.1.100
s=Example Movie
i=A sample H.264/AAC movie
u=http://example.com/info.html
e=admin@example.com
c=IN IP4 0.0.0.0
t=0 0
a=recvonly
a=control:rtsp://example.com/movie/
a=range:npt=0-3600.0
a=etag:abc123def456
m=video 0 RTP/AVP 96
a=rtpmap:96 H264/90000
a=fmtp:96 profile-level-id=42e01f; packetization-mode=1
a=control:trackID=1
a=framerate:30
m=audio 0 RTP/AVP 97
a=rtpmap:97 mpeg4-generic/44100/2
a=fmtp:97 streamtype=5; profile-level-id=1; mode=AAC-hbr;
          sizelength=13; indexlength=3; indexdeltalength=3
a=control:trackID=2
```

---

## Parsing SDP for RTSP

### Pseudocode

```python
def parse_sdp_for_rtsp(sdp_text, base_url):
    session = {
        'control': None,
        'range': None,
        'streams': []
    }

    current_stream = None

    for line in sdp_text.split('\n'):
        type_char = line[0] if line else ''
        value = line[2:].strip() if len(line) > 2 else ''

        if type_char == 'm':
            # New media stream
            parts = value.split()
            current_stream = {
                'type': parts[0],        # audio, video
                'port': int(parts[1]),
                'protocol': parts[2],    # RTP/AVP
                'formats': parts[3:],
                'control': None,
                'rtpmap': {},
                'fmtp': {}
            }
            session['streams'].append(current_stream)

        elif type_char == 'a':
            # Attribute
            if '=' in value:
                attr_name, attr_value = value.split('=', 1)
            else:
                attr_name, attr_value = value, None

            if attr_name == 'control':
                url = resolve_url(base_url, attr_value)
                if current_stream:
                    current_stream['control'] = url
                else:
                    session['control'] = url

            elif attr_name == 'range':
                session['range'] = parse_range(attr_value)

            elif attr_name == 'rtpmap':
                pt, encoding = parse_rtpmap(attr_value)
                if current_stream:
                    current_stream['rtpmap'][pt] = encoding

            elif attr_name == 'fmtp':
                pt, params = parse_fmtp(attr_value)
                if current_stream:
                    current_stream['fmtp'][pt] = params

    return session
```

---

## Common SDP Attributes for RTSP

| Attribute | Level | Description |
|-----------|-------|-------------|
| `a=control:` | Session/Media | RTSP control URL |
| `a=range:` | Session | Presentation duration |
| `a=etag:` | Session | Version tag |
| `a=rtpmap:` | Media | Payload type mapping |
| `a=fmtp:` | Media | Format parameters |
| `a=framerate:` | Media | Video frame rate |
| `a=recvonly` | Session | Playback only |
| `a=sendonly` | Session | Recording only |

---

> [Back to Index](00-index.md) | [Previous: RTP Interaction](B-rtp-interaction.md) | [Next: Minimal Implementation](D-minimal-implementation.md)
