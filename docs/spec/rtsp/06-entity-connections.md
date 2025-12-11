# Sections 8-9: Entity & Connections

> [Back to Index](00-index.md) | [Previous: Request/Response](05-request-response.md) | [Next: Methods](07-methods.md)

## 8. Entity

An entity is the payload transferred with a message. For header details, see [Headers](09-headers.md).

### 8.1 Entity Header Fields

| Header | Description | Example |
|--------|-------------|---------|
| `Allow` | Allowed methods | `Allow: SETUP, PLAY, TEARDOWN` |
| `Content-Base` | Base URL for entity | `Content-Base: rtsp://server/` |
| `Content-Encoding` | Content encoding | `Content-Encoding: gzip` |
| `Content-Language` | Language | `Content-Language: en` |
| `Content-Length` | Body length in bytes | `Content-Length: 376` |
| `Content-Location` | Entity location | `Content-Location: /movie.sdp` |
| `Content-Type` | Media type | `Content-Type: application/sdp` |
| `Expires` | Expiration date | `Expires: Thu, 01 Dec 1994 16:00:00 GMT` |
| `Last-Modified` | Last modification | `Last-Modified: Tue, 15 Nov 1994 12:45:26 GMT` |

### 8.2 Entity Body

The entity body follows the blank line after headers.

### Common Entity Types

| Method | Direction | Content-Type | Purpose |
|--------|-----------|--------------|---------|
| [DESCRIBE](07-methods.md#102-describe) | Response | `application/sdp` | [Session description](C-sdp-usage.md) |
| [ANNOUNCE](07-methods.md#103-announce) | Request/Response | `application/sdp` | Presentation description |
| [GET_PARAMETER](07-methods.md#108-get_parameter) | Response | `text/parameters` | Parameter values |
| [SET_PARAMETER](07-methods.md#109-set_parameter) | Request | `text/parameters` | Parameter settings |

### Entity Body Example

```
RTSP/1.0 200 OK
CSeq: 2
Content-Type: application/sdp
Content-Length: 233

v=0
o=- 2890844526 2890842807 IN IP4 192.168.1.1
s=Example Session
i=A streaming example
t=0 0
a=control:rtsp://server/movie/
m=video 0 RTP/AVP 96
a=rtpmap:96 H264/90000
a=control:trackID=1
```

---

## 9. Connections

### Transport Protocol Options

| Protocol | Description | Default Port |
|----------|-------------|--------------|
| TCP | Reliable, ordered | 554 |
| UDP | Unreliable, faster | 554 |

### TCP Connections

- **Persistent by default** (unlike HTTP/1.0)
- Multiple requests/responses on same connection
- Session can span multiple connections
- Client/server both may open connections

### UDP Connections

- Request/response must fit in single datagram
- Retransmission responsibility on client
- Timeout recommended: 0.5s base, double on retry

---

## 9.1 Pipelining

Multiple requests can be sent without waiting for responses. See [Examples](12-examples.md) for pipelining in action.

### Rules

1. Requests MUST be processed in order received
2. Server MUST send responses in order
3. Client MUST NOT assume specific timing

### Example

```
C->S: PLAY rtsp://server/movie RTSP/1.0
      CSeq: 5
      Session: 12345678
      Range: npt=10-15

C->S: PLAY rtsp://server/movie RTSP/1.0
      CSeq: 6
      Session: 12345678
      Range: npt=20-25

[Server queues both, executes in order]

S->C: RTSP/1.0 200 OK
      CSeq: 5
      Range: npt=10-15

S->C: RTSP/1.0 200 OK
      CSeq: 6
      Range: npt=20-25
```

---

## 9.2 Reliability and Acknowledgements

### Over TCP

- TCP provides reliability
- No application-level retransmission needed
- Responses acknowledge requests via CSeq

### Over UDP

RTSP over UDP (rtspu://) requires application-level reliability:

1. **Request Retransmission**
   - Start with 0.5 second timeout
   - Double timeout on each retry
   - Maximum recommended retries: 7

2. **Response Acknowledgement**
   - Response acts as acknowledgement
   - Match response CSeq to request

### UDP Reliability Algorithm

```
timeout = 0.5 seconds
max_retries = 7
retries = 0

while not_acknowledged and retries < max_retries:
    send_request()
    wait(timeout)
    if response_received:
        not_acknowledged = false
    else:
        timeout = timeout * 2
        retries++
```

---

## Connection Management

### Keep-Alive

Default behavior is to keep connections open.

```
Connection: Keep-Alive    // Explicit (optional)
```

### Close Connection

```
Connection: close         // Close after response
```

### Session vs Connection

| Concept | Scope | Identifier |
|---------|-------|------------|
| Connection | Transport layer | TCP socket |
| Session | RTSP protocol | Session header |

**Important**: An RTSP session can span multiple TCP connections.

```
Connection 1:
  SETUP -> Session created (ID: 12345678)

[Connection 1 closes]

Connection 2:
  PLAY with Session: 12345678 -> Resumes session
```

### Session Timeout

Sessions expire without activity. See [State Machines](A-state-machines.md) for timeout behavior.

```
Session: 12345678;timeout=60
```

- Default timeout: 60 seconds
- Client must send keepalive before timeout
- [OPTIONS](07-methods.md#101-options) or [GET_PARAMETER](07-methods.md#108-get_parameter) work as keepalive

### Keepalive Example

```
C->S: OPTIONS rtsp://server/movie RTSP/1.0
      CSeq: 100
      Session: 12345678

S->C: RTSP/1.0 200 OK
      CSeq: 100
      Session: 12345678
```

---

## Interleaved Binary Data

Media data can be sent over the RTSP connection (TCP only).

### Frame Format

```
$<channel><length><data>
```

| Field | Size | Description |
|-------|------|-------------|
| `$` | 1 byte | Magic marker (0x24) |
| channel | 1 byte | Channel number (0-255) |
| length | 2 bytes | Data length (big-endian) |
| data | variable | Actual payload |

### Channel Assignment

Specified in Transport header:

```
Transport: RTP/AVP/TCP;interleaved=0-1
```

- Channel 0: RTP data
- Channel 1: RTCP data

See [Section 10.12](07-methods.md#1012-embedded-interleaved-binary-data) for details.

---

## Connection Summary

| Aspect | TCP | UDP |
|--------|-----|-----|
| Reliability | Built-in | Application-level |
| Multiple requests | Pipelining supported | One at a time |
| Media interleaving | Supported | Not applicable |
| Default | Yes | No |
| Session persistence | Across connections | Per-datagram |

---

> [Back to Index](00-index.md) | [Previous: Request/Response](05-request-response.md) | [Next: Methods](07-methods.md)
