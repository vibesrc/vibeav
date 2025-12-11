# Section 12: Header Field Definitions

> [Back to Index](00-index.md) | [Previous: Status Codes](08-status-codes.md) | [Next: Transport Header](10-transport-header.md)

## Header Overview

Headers are categorized by type and support requirements.

### Header Types

| Type | Code | Scope |
|------|------|-------|
| General | g | Request & Response |
| Request | R | Request only |
| Response | r | Response only |
| Entity | e | Message body metadata |

### Support Levels

| Level | Meaning |
|-------|---------|
| req. | MUST implement |
| opt. | MAY implement |

---

## Header Reference Table

| Header | Type | Support | Methods |
|--------|------|---------|---------|
| Accept | R | opt. | entity |
| Accept-Encoding | R | opt. | entity |
| Accept-Language | R | opt. | all |
| Allow | r | opt. | all |
| Authorization | R | opt. | all |
| Bandwidth | R | opt. | all |
| Blocksize | R | opt. | all (not SETUP) |
| Cache-Control | g | opt. | SETUP |
| Conference | R | opt. | SETUP |
| Connection | g | req. | all |
| Content-Base | e | opt. | entity |
| Content-Encoding | e | req. | SET_PARAMETER |
| Content-Language | e | req. | all |
| Content-Length | e | req. | SET_PARAMETER, ANNOUNCE |
| Content-Location | e | opt. | entity |
| Content-Type | e | req. | SET_PARAMETER, ANNOUNCE |
| CSeq | g | **req.** | all |
| Date | g | opt. | all |
| Expires | e | opt. | DESCRIBE, ANNOUNCE |
| From | R | opt. | all |
| Host | R | opt. | all |
| If-Match | R | opt. | all |
| If-Modified-Since | R | opt. | DESCRIBE, SETUP |
| Last-Modified | e | opt. | entity |
| Location | r | opt. | 3xx |
| Proxy-Authenticate | r | req. | all |
| Proxy-Require | R | req. | all |
| Public | r | opt. | all |
| Range | R,r | opt. | PLAY, PAUSE, RECORD |
| Referer | R | opt. | all |
| Require | R | req. | all |
| Retry-After | r | opt. | 503 |
| RTP-Info | r | req. | PLAY |
| Scale | R,r | opt. | PLAY |
| Server | r | opt. | all |
| Session | R,r | **req.** | all (after SETUP) |
| Speed | R,r | opt. | PLAY |
| Timestamp | g | opt. | all |
| Transport | R,r | **req.** | SETUP |
| Unsupported | r | req. | all |
| User-Agent | R | opt. | all |
| Vary | r | opt. | all |
| Via | g | opt. | all |
| WWW-Authenticate | r | opt. | all |

---

## Essential Headers

### CSeq (Required)

Sequence number for request/response matching.

```
CSeq: 312
```

- Integer, starts at any value
- Increments per request
- Response MUST echo request CSeq

### Session (Required after SETUP)

Session identifier.

```
Session: 12345678
Session: 12345678;timeout=60
```

- Server generates on first SETUP
- Client includes in subsequent requests
- Optional timeout parameter (seconds)

### Transport (Required in SETUP)

Transport specification. **This is critical for implementation** - see dedicated [Transport Header](10-transport-header.md) documentation.

```
Transport: RTP/AVP;unicast;client_port=4588-4589
```

---

## Content Headers

### Accept

Acceptable response content types.

```
Accept: application/sdp, application/rtsl
Accept: application/sdp;level=1, application/sdp
```

### Content-Type

Media type of body.

```
Content-Type: application/sdp
Content-Type: text/parameters
```

### Content-Length

Body size in bytes.

```
Content-Length: 376
```

**Required** when body present on persistent connection.

### Content-Base

Base URL for relative URLs in body.

```
Content-Base: rtsp://server.example.com/movie/
```

### Content-Encoding

Encoding applied to body.

```
Content-Encoding: gzip
```

### Content-Language

Natural language of content.

```
Content-Language: en
Content-Language: en-US, fr-CA
```

---

## Playback Control Headers

### Range

Time range for PLAY/PAUSE/RECORD. See [Protocol Parameters](03-protocol-parameters.md) for time format details.

```
Range: npt=0-                    ; Start to end
Range: npt=10.5-20.0             ; 10.5 to 20 seconds
Range: smpte=0:10:20-0:20:00     ; SMPTE timecode
Range: clock=19970123T153600Z-  ; Absolute time
```

### Scale

Playback speed factor.

```
Scale: 1         ; Normal speed (default)
Scale: 2         ; 2x fast forward
Scale: -1        ; Normal speed reverse
Scale: 0.5       ; Half speed
```

### Speed

Delivery speed (not presentation speed).

```
Speed: 2.5
```

Requests 2.5x delivery rate. Server may ignore if unsupported.

### RTP-Info

RTP synchronization info in PLAY response. See [RTP Interaction](B-rtp-interaction.md) for usage details.

```
RTP-Info: url=rtsp://server/movie/trackID=1;seq=12345;rtptime=3450012,
          url=rtsp://server/movie/trackID=2;seq=54321;rtptime=2876543
```

Parameters:
- `url` - Stream URL
- `seq` - Initial RTP sequence number
- `rtptime` - Initial RTP timestamp

---

## Session Management Headers

### Bandwidth

Estimated available bandwidth (bps).

```
Bandwidth: 4000000
```

Client hint for server stream selection.

### Blocksize

Preferred media packet size.

```
Blocksize: 1500
```

### Conference

Conference identifier to join.

```
Conference: 128.16.64.19/32492374
```

### Require

Required features for request.

```
Require: implicit-play
```

Server returns 551 if unsupported.

### Proxy-Require

Required proxy features.

```
Proxy-Require: gzipped-messages
```

### Unsupported

Features not supported (in 551 response).

```
Unsupported: implicit-play
```

---

## Cache Control Headers

### Cache-Control

Caching directives.

```
Cache-Control: no-cache
Cache-Control: public, max-age=3600
```

Directives:
| Directive | Meaning |
|-----------|---------|
| `no-cache` | Must revalidate |
| `public` | Cacheable by any cache |
| `private` | Only client can cache |
| `no-transform` | No transformation |
| `only-if-cached` | Only from cache |
| `max-stale` | Accept stale content |
| `min-fresh` | Require freshness |
| `must-revalidate` | Must check origin |

### Expires

Expiration date for content.

```
Expires: Thu, 01 Dec 1994 16:00:00 GMT
```

### Last-Modified

Last modification date.

```
Last-Modified: Tue, 15 Nov 1994 12:45:26 GMT
```

### If-Modified-Since

Conditional request.

```
If-Modified-Since: Tue, 15 Nov 1994 12:45:26 GMT
```

### If-Match

Conditional based on ETag.

```
If-Match: "xyzzy"
```

---

## Identification Headers

### User-Agent

Client software info.

```
User-Agent: MyPlayer/1.0 (Linux)
```

### Server

Server software info.

```
Server: ExampleServer/2.0
```

### From

User email (for logging).

```
From: user@example.com
```

### Referer

Referring URL.

```
Referer: http://www.example.com/video.html
```

---

## Authentication Headers

### Authorization

Client credentials.

```
Authorization: Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==
Authorization: Digest username="user", realm="example", ...
```

### WWW-Authenticate

Server auth challenge.

```
WWW-Authenticate: Basic realm="Streaming Server"
WWW-Authenticate: Digest realm="example", nonce="abc123", ...
```

### Proxy-Authenticate

Proxy auth challenge.

```
Proxy-Authenticate: Basic realm="Proxy"
```

### Proxy-Authorization

Proxy credentials.

```
Proxy-Authorization: Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==
```

---

## Other Headers

### Connection

Connection options.

```
Connection: close
Connection: Keep-Alive
```

### Date

Message generation time.

```
Date: Sat, 23 Nov 1996 17:05:33 GMT
```

### Public

Supported methods (in OPTIONS response).

```
Public: DESCRIBE, SETUP, TEARDOWN, PLAY, PAUSE
```

### Allow

Methods allowed for resource.

```
Allow: SETUP, PLAY, TEARDOWN
```

### Location

Redirect destination.

```
Location: rtsp://newserver.example.com/movie
```

### Retry-After

When to retry (with 503).

```
Retry-After: 120
Retry-After: Fri, 31 Dec 1999 23:59:59 GMT
```

### Timestamp

Request-response timing.

```
Timestamp: 1234567890
Timestamp: 1234567890 0.321
```

Second value is server processing delay.

### Via

Proxy chain.

```
Via: 1.0 proxy1.example.com, 1.0 proxy2.example.com
```

### Vary

Response varies by these headers.

```
Vary: Accept-Language
```

---

## Header Usage by Method

| Method | Key Headers |
|--------|-------------|
| OPTIONS | Public (response) |
| DESCRIBE | Accept, Content-Type, Content-Base |
| ANNOUNCE | Content-Type, Content-Length |
| SETUP | Transport, Session |
| PLAY | Session, Range, RTP-Info |
| PAUSE | Session, Range |
| TEARDOWN | Session |
| GET_PARAMETER | Session, Content-Type |
| SET_PARAMETER | Session, Content-Type |
| RECORD | Session, Range |

---

> [Back to Index](00-index.md) | [Previous: Status Codes](08-status-codes.md) | [Next: Transport Header](10-transport-header.md)
