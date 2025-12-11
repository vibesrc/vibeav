# Section 10: Method Definitions

> [Back to Index](00-index.md) | [Previous: Entity & Connections](06-entity-connections.md) | [Next: Status Codes](08-status-codes.md)

## Method Overview

| Method | Direction | Object | Requirement |
|--------|-----------|--------|-------------|
| OPTIONS | C↔S | P, S | Required (S→C optional) |
| DESCRIBE | C→S | P, S | Recommended |
| ANNOUNCE | C↔S | P, S | Optional |
| SETUP | C→S | S | **Required** |
| PLAY | C→S | P, S | **Required** |
| PAUSE | C→S | P, S | Recommended |
| TEARDOWN | C→S | P, S | **Required** |
| GET_PARAMETER | C↔S | P, S | Optional |
| SET_PARAMETER | C↔S | P, S | Optional |
| REDIRECT | S→C | P, S | Optional |
| RECORD | C→S | P, S | Optional |

**Legend**: C=Client, S=Server, P=Presentation, S=Stream

---

## 10.1 OPTIONS

Query supported methods and capabilities.

### Characteristics

| Attribute | Value |
|-----------|-------|
| Direction | C→S, S→C |
| Affects state | No |
| Body | None |

### Use Cases

1. Check server capabilities before session
2. Keep session alive (heartbeat)
3. Test server liveness

### Request

```
OPTIONS rtsp://server.example.com/movie RTSP/1.0
CSeq: 1
Require: implicit-play
Proxy-Require: gzipped-messages
```

### Response

```
RTSP/1.0 200 OK
CSeq: 1
Public: DESCRIBE, SETUP, TEARDOWN, PLAY, PAUSE
```

The `Public` header lists supported methods. See [Headers](09-headers.md) for details.

### Server-Wide OPTIONS

```
OPTIONS * RTSP/1.0
CSeq: 1
```

Returns capabilities for entire server, not specific resource.

---

## 10.2 DESCRIBE

Retrieve presentation description (typically SDP).

### Characteristics

| Attribute | Value |
|-----------|-------|
| Direction | C→S |
| Affects state | No |
| Response body | Presentation description |

### Request Headers

| Header | Purpose |
|--------|---------|
| `Accept` | Acceptable formats (e.g., `application/sdp`) |

### Request

```
DESCRIBE rtsp://server.example.com/movie RTSP/1.0
CSeq: 2
Accept: application/sdp
```

### Response

```
RTSP/1.0 200 OK
CSeq: 2
Content-Type: application/sdp
Content-Length: 460
Content-Base: rtsp://server.example.com/movie/

v=0
o=- 2890844526 2890842807 IN IP4 192.168.1.1
s=Movie Title
i=A sample movie
t=0 0
a=control:*
m=video 0 RTP/AVP 96
a=rtpmap:96 H264/90000
a=control:trackID=1
m=audio 0 RTP/AVP 97
a=rtpmap:97 mpeg4-generic/44100/2
a=control:trackID=2
```

### Important Notes

- Response MUST contain complete media initialization info
- DESCRIBE is NOT the only way to get description (HTTP, email, etc.)
- If description obtained elsewhere, client SHOULD NOT call DESCRIBE

---

## 10.3 ANNOUNCE

Post or update session description.

### Characteristics

| Attribute | Value |
|-----------|-------|
| Direction | C→S (post), S→C (update) |
| Affects state | No |
| Request body | Presentation description |

### Client→Server (Post new content)

```
ANNOUNCE rtsp://server.example.com/meeting RTSP/1.0
CSeq: 3
Session: 47112344
Content-Type: application/sdp
Content-Length: 332

v=0
o=user 2890844526 2890845468 IN IP4 126.16.64.4
s=Meeting Recording
...
```

### Server→Client (Live update)

```
ANNOUNCE rtsp://server.example.com/live RTSP/1.0
CSeq: 3
Session: 47112344
Content-Type: application/sdp
Content-Length: 420

v=0
o=- 2890844526 2890845469 IN IP4 126.16.64.4
s=Live Stream
...
```

When streams change, send complete description (not just changes).

---

## 10.4 SETUP

Establish transport parameters for a stream.

### Characteristics

| Attribute | Value |
|-----------|-------|
| Direction | C→S |
| Affects state | **Yes** (Init → Ready) |
| Required headers | Transport |

### Request

```
SETUP rtsp://server.example.com/movie/trackID=1 RTSP/1.0
CSeq: 3
Transport: RTP/AVP;unicast;client_port=4588-4589
```

### Response

```
RTSP/1.0 200 OK
CSeq: 3
Session: 12345678;timeout=60
Transport: RTP/AVP;unicast;client_port=4588-4589;server_port=6256-6257
```

### Key Points

1. **Session Creation**: First SETUP creates session, returns Session ID
2. **Adding Streams**: Subsequent SETUPs with same Session add streams
3. **Changing Transport**: SETUP on playing stream may change parameters
4. **Per-Stream**: SETUP targets stream URL, not presentation URL

### Transport Negotiation

Client offers transport options:
```
Transport: RTP/AVP;unicast;client_port=4588-4589,
           RTP/AVP/TCP;interleaved=0-1
```

Server selects ONE:
```
Transport: RTP/AVP;unicast;client_port=4588-4589;server_port=6256-6257
```

See [Transport Header](10-transport-header.md) for full Transport header details.

---

## 10.5 PLAY

Start media delivery.

### Characteristics

| Attribute | Value |
|-----------|-------|
| Direction | C→S |
| Affects state | **Yes** (Ready → Playing) |
| Prerequisite | SETUP completed |

### Request

```
PLAY rtsp://server.example.com/movie RTSP/1.0
CSeq: 4
Session: 12345678
Range: npt=0-
```

### Response

```
RTSP/1.0 200 OK
CSeq: 4
Session: 12345678
Range: npt=0-634.10
RTP-Info: url=rtsp://server.example.com/movie/trackID=1;
          seq=12345;rtptime=3450012,
          url=rtsp://server.example.com/movie/trackID=2;
          seq=54321;rtptime=2876543
```

### Range Examples

See [Protocol Parameters](03-protocol-parameters.md) for full time format details.

| Range | Meaning |
|-------|---------|
| `npt=0-` | From start to end |
| `npt=10-20` | From 10s to 20s |
| `npt=now-` | From current position (live) |
| `smpte=0:10:00-` | From timecode 10 minutes |

### Pipelined PLAY Requests

PLAY requests queue and execute in order:

```
PLAY rtsp://server/movie RTSP/1.0
CSeq: 5
Session: 12345678
Range: npt=10-15

PLAY rtsp://server/movie RTSP/1.0
CSeq: 6
Session: 12345678
Range: npt=20-25
```

Result: Plays 10-15, then immediately 20-25.

### PLAY Without Range

- If paused: Resume from pause point
- If not started: Start from beginning
- If playing: No effect (can test liveness)

---

## 10.6 PAUSE

Temporarily halt media delivery.

### Characteristics

| Attribute | Value |
|-----------|-------|
| Direction | C→S |
| Affects state | **Yes** (Playing → Ready) |
| Preserves | Session and pause point |

### Request

```
PAUSE rtsp://server.example.com/movie RTSP/1.0
CSeq: 5
Session: 12345678
```

### Response

```
RTSP/1.0 200 OK
CSeq: 5
Session: 12345678
Range: npt=45.27-
```

### PAUSE with Range

Request pause at specific point (future):

```
PAUSE rtsp://server.example.com/movie RTSP/1.0
CSeq: 6
Session: 12345678
Range: npt=60
```

Pauses when NPT reaches 60 seconds.

### Aggregate vs Stream PAUSE

- **Aggregate URL**: Pauses all streams
- **Stream URL**: May be rejected with 460 (Only Aggregate Operation Allowed)

---

## 10.7 TEARDOWN

Stop session and free resources.

### Characteristics

| Attribute | Value |
|-----------|-------|
| Direction | C→S |
| Affects state | **Yes** (Any → closed) |
| Frees | Server and client resources |

### Request

```
TEARDOWN rtsp://server.example.com/movie RTSP/1.0
CSeq: 7
Session: 12345678
```

### Response

```
RTSP/1.0 200 OK
CSeq: 7
```

### Notes

- Session ID becomes invalid after TEARDOWN
- Server SHOULD free all session resources
- Connection may remain open for new sessions

---

## 10.8 GET_PARAMETER

Retrieve parameter values from server/stream.

### Characteristics

| Attribute | Value |
|-----------|-------|
| Direction | C↔S |
| Affects state | No |
| Body | Parameter names (request), values (response) |

### Empty Body (Keepalive)

```
GET_PARAMETER rtsp://server.example.com/movie RTSP/1.0
CSeq: 8
Session: 12345678
```

```
RTSP/1.0 200 OK
CSeq: 8
Session: 12345678
```

### With Parameters

Request:
```
GET_PARAMETER rtsp://server.example.com/movie RTSP/1.0
CSeq: 9
Session: 12345678
Content-Type: text/parameters
Content-Length: 26

jitter
packet_loss
```

Response:
```
RTSP/1.0 200 OK
CSeq: 9
Session: 12345678
Content-Type: text/parameters
Content-Length: 30

jitter: 120ms
packet_loss: 0.5%
```

---

## 10.9 SET_PARAMETER

Set parameter values.

### Characteristics

| Attribute | Value |
|-----------|-------|
| Direction | C↔S |
| Affects state | No |
| Body | Parameter name-value pairs |

### Request

```
SET_PARAMETER rtsp://server.example.com/movie RTSP/1.0
CSeq: 10
Session: 12345678
Content-Type: text/parameters
Content-Length: 20

barparam: barstuff
```

### Response

```
RTSP/1.0 200 OK
CSeq: 10
Session: 12345678
```

### Error: Read-Only Parameter

```
RTSP/1.0 458 Parameter Is Read-Only
CSeq: 10
```

### Best Practice

- Set ONE parameter per request
- Allows atomic failure handling

---

## 10.10 REDIRECT

Server redirects client to new location.

### Characteristics

| Attribute | Value |
|-----------|-------|
| Direction | S→C |
| Affects state | No (client must TEARDOWN) |

### Immediate Redirect

```
REDIRECT rtsp://server.example.com/movie RTSP/1.0
CSeq: 11
Session: 12345678
Location: rtsp://newserver.example.com/movie
```

### Scheduled Redirect

```
REDIRECT rtsp://server.example.com/movie RTSP/1.0
CSeq: 12
Session: 12345678
Location: rtsp://newserver.example.com/movie
Range: npt=300
```

Client should redirect when reaching 300 seconds.

---

## 10.11 RECORD

Initiate recording of media.

### Characteristics

| Attribute | Value |
|-----------|-------|
| Direction | C→S |
| Affects state | **Yes** (Ready → Recording) |
| Optional | Yes |

### Request

```
RECORD rtsp://server.example.com/meeting RTSP/1.0
CSeq: 11
Session: 12345678
Range: npt=0-
```

### Response

```
RTSP/1.0 200 OK
CSeq: 11
Session: 12345678
```

### Low Storage Warning

```
RTSP/1.0 250 Low on Storage Space
CSeq: 12
Session: 12345678
Range: npt=0-3600
```

Server may only be able to record 1 hour.

---

## 10.12 Embedded (Interleaved) Binary Data

Stream RTP/RTCP data over RTSP TCP connection.

### Use Cases

- Firewall traversal (only port 554 needed)
- NAT traversal
- Simplified deployment

### Frame Format

```
+--------+--------+--------+--------+--------+...
|  '$'   |channel |  length (16-bit)  |  data
| (0x24) | (8-bit)|   (big-endian)    |
+--------+--------+--------+--------+--------+...
```

### Channel Assignment

```
Transport: RTP/AVP/TCP;interleaved=0-1
```

- Channel 0: RTP data
- Channel 1: RTCP data

### Example SETUP for Interleaved

```
SETUP rtsp://server.example.com/movie/trackID=1 RTSP/1.0
CSeq: 3
Transport: RTP/AVP/TCP;unicast;interleaved=0-1
```

```
RTSP/1.0 200 OK
CSeq: 3
Session: 12345678
Transport: RTP/AVP/TCP;unicast;interleaved=0-1
```

### Multiple Streams

```
Stream 1: Transport: RTP/AVP/TCP;unicast;interleaved=0-1
Stream 2: Transport: RTP/AVP/TCP;unicast;interleaved=2-3
```

### Parsing Interleaved Data

```
while connected:
    byte = read(1)
    if byte == '$':
        channel = read(1)
        length = read_uint16_be()
        data = read(length)
        handle_rtp_rtcp(channel, data)
    elif byte == 'R':
        # Start of "RTSP" - response message
        parse_rtsp_message()
```

---

## Method Summary

### Required for Playback

| Method | Purpose |
|--------|---------|
| SETUP | Establish transport |
| PLAY | Start delivery |
| TEARDOWN | End session |

### Recommended

| Method | Purpose |
|--------|---------|
| DESCRIBE | Get presentation info |
| PAUSE | Temporary halt |

### Optional

| Method | Purpose |
|--------|---------|
| OPTIONS | Query capabilities |
| ANNOUNCE | Post/update description |
| GET_PARAMETER | Get values / keepalive |
| SET_PARAMETER | Set values |
| RECORD | Start recording |
| REDIRECT | Redirect client |

---

> [Back to Index](00-index.md) | [Previous: Entity & Connections](06-entity-connections.md) | [Next: Status Codes](08-status-codes.md)
