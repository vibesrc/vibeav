# Sections 6-7: Request and Response

> [Back to Index](00-index.md) | [Previous: Message Format](04-message-format.md) | [Next: Entity & Connections](06-entity-connections.md)

For method-specific details, see [Methods](07-methods.md). For header details, see [Headers](09-headers.md).

## 6. Request

### 6.1 Request Line

```
Request-Line = Method SP Request-URI SP RTSP-Version CRLF
```

### Components

| Component | Description | Example |
|-----------|-------------|---------|
| Method | Action to perform | `PLAY`, `SETUP`, `DESCRIBE` |
| SP | Single space | ` ` |
| Request-URI | Target resource | `rtsp://server.com/movie` |
| RTSP-Version | Protocol version | `RTSP/1.0` |

### Request-URI Forms

| Form | Use Case | Example |
|------|----------|---------|
| Absolute URI | Normal requests | `rtsp://server/movie` |
| Asterisk (*) | Server-wide request | `OPTIONS * RTSP/1.0` |

### Complete Request Example

```
DESCRIBE rtsp://server.example.com/movie RTSP/1.0
CSeq: 1
Accept: application/sdp
```

---

## 6.2 Request Header Fields

Headers specific to requests:

| Header | Required | Description |
|--------|----------|-------------|
| `Accept` | opt | Acceptable media types |
| `Accept-Encoding` | opt | Acceptable encodings |
| `Accept-Language` | opt | Preferred languages |
| `Authorization` | opt | Authentication credentials |
| `Bandwidth` | opt | Estimated bandwidth |
| `Blocksize` | opt | Preferred packet size |
| `From` | opt | User email address |
| `If-Match` | opt | Conditional request |
| `If-Modified-Since` | opt | Conditional request |
| `Proxy-Authorization` | opt | Proxy credentials |
| `Proxy-Require` | opt | Required proxy features |
| `Range` | opt | Time range request |
| `Referer` | opt | Referring URL |
| `Require` | opt | Required features |
| `Scale` | opt | Playback speed ratio |
| `Session` | req* | Session identifier |
| `Speed` | opt | Delivery speed |
| `Transport` | req** | [Transport specification](10-transport-header.md) |
| `User-Agent` | opt | Client software info |

\* Required after [SETUP](07-methods.md#104-setup)
\** Required in [SETUP](07-methods.md#104-setup)

---

## 7. Response

### 7.1 Status-Line

```
Status-Line = RTSP-Version SP Status-Code SP Reason-Phrase CRLF
```

### Components

| Component | Description | Example |
|-----------|-------------|---------|
| RTSP-Version | Protocol version | `RTSP/1.0` |
| Status-Code | 3-digit result code | `200` |
| Reason-Phrase | Human-readable text | `OK` |

### Complete Response Example

```
RTSP/1.0 200 OK
CSeq: 1
Date: 23 Jan 1997 15:35:06 GMT
Content-Type: application/sdp
Content-Length: 376

v=0
o=- 2890844526 2890842807 IN IP4 126.16.64.4
s=SDP Seminar
...
```

---

## 7.1.1 Status Codes

### Status Code Classes

| Class | Range | Meaning |
|-------|-------|---------|
| 1xx | 100-199 | Informational |
| 2xx | 200-299 | Success |
| 3xx | 300-399 | Redirection |
| 4xx | 400-499 | Client Error |
| 5xx | 500-599 | Server Error |

### Common Status Codes

#### Success (2xx)

| Code | Phrase | Description |
|------|--------|-------------|
| 200 | OK | Request succeeded |
| 250 | Low on Storage Space | Recording may fail (RECORD) |

#### Redirection (3xx)

| Code | Phrase | Description |
|------|--------|-------------|
| 301 | Moved Permanently | Resource moved |
| 302 | Moved Temporarily | Resource temporarily moved |
| 303 | See Other | Use different URI |
| 305 | Use Proxy | Use specified proxy |

#### Client Error (4xx)

| Code | Phrase | Description |
|------|--------|-------------|
| 400 | Bad Request | Malformed request |
| 401 | Unauthorized | Authentication required |
| 403 | Forbidden | Access denied |
| 404 | Not Found | Resource not found |
| 405 | Method Not Allowed | Method not supported |
| 406 | Not Acceptable | Can't meet Accept headers |
| 408 | Request Timeout | Request timed out |
| 451 | Parameter Not Understood | Unknown parameter |
| 452 | Conference Not Found | Unknown conference |
| 453 | Not Enough Bandwidth | Insufficient bandwidth |
| 454 | Session Not Found | Unknown/expired session |
| 455 | Method Not Valid in This State | Wrong state for method |
| 456 | Header Field Not Valid for Resource | Invalid header for resource |
| 457 | Invalid Range | Range out of bounds |
| 458 | Parameter Is Read-Only | Can't modify parameter |
| 459 | Aggregate Operation Not Allowed | Use stream URL instead |
| 460 | Only Aggregate Operation Allowed | Use presentation URL |
| 461 | Unsupported Transport | Transport not supported |
| 462 | Destination Unreachable | Can't reach destination |

#### Server Error (5xx)

| Code | Phrase | Description |
|------|--------|-------------|
| 500 | Internal Server Error | Server error |
| 501 | Not Implemented | Method not implemented |
| 502 | Bad Gateway | Upstream error |
| 503 | Service Unavailable | Temporarily unavailable |
| 504 | Gateway Timeout | Upstream timeout |
| 505 | RTSP Version Not Supported | Version mismatch |
| 551 | Option Not Supported | Feature not supported |

See [Status Codes](08-status-codes.md) for detailed descriptions.

---

## 7.1.2 Response Header Fields

Headers specific to responses:

| Header | Description |
|--------|-------------|
| `Location` | Redirect destination |
| `Proxy-Authenticate` | Proxy auth challenge |
| `Public` | Supported methods |
| `Retry-After` | When to retry |
| `RTP-Info` | RTP sync information |
| `Server` | Server software info |
| `Unsupported` | Unsupported features |
| `Vary` | Content negotiation |
| `WWW-Authenticate` | Auth challenge |

---

## Request/Response Examples

### OPTIONS Request

```
C->S: OPTIONS rtsp://server.example.com/movie RTSP/1.0
      CSeq: 1

S->C: RTSP/1.0 200 OK
      CSeq: 1
      Public: DESCRIBE, SETUP, TEARDOWN, PLAY, PAUSE
```

### DESCRIBE Request

```
C->S: DESCRIBE rtsp://server.example.com/movie RTSP/1.0
      CSeq: 2
      Accept: application/sdp

S->C: RTSP/1.0 200 OK
      CSeq: 2
      Content-Type: application/sdp
      Content-Length: 460

      v=0
      o=- 2890844526 2890842807 IN IP4 127.0.0.1
      s=Movie
      t=0 0
      a=control:*
      m=video 0 RTP/AVP 96
      a=rtpmap:96 H264/90000
      a=control:trackID=1
      m=audio 0 RTP/AVP 97
      a=rtpmap:97 MPEG4-GENERIC/44100/2
      a=control:trackID=2
```

### SETUP Request

```
C->S: SETUP rtsp://server.example.com/movie/trackID=1 RTSP/1.0
      CSeq: 3
      Transport: RTP/AVP;unicast;client_port=4588-4589

S->C: RTSP/1.0 200 OK
      CSeq: 3
      Session: 12345678;timeout=60
      Transport: RTP/AVP;unicast;client_port=4588-4589;server_port=6256-6257
```

### Error Response

```
C->S: PLAY rtsp://server.example.com/movie RTSP/1.0
      CSeq: 4

S->C: RTSP/1.0 454 Session Not Found
      CSeq: 4
```

---

> [Back to Index](00-index.md) | [Previous: Message Format](04-message-format.md) | [Next: Entity & Connections](06-entity-connections.md)
