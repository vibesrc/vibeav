# Section 11: Status Code Definitions

> [Back to Index](00-index.md) | [Previous: Methods](07-methods.md) | [Next: Headers](09-headers.md)

## Status Code Overview

Status codes follow HTTP conventions with RTSP-specific additions.

### Status Code Classes

| Class | Range | Meaning | Action |
|-------|-------|---------|--------|
| 1xx | 100-199 | Informational | Continue waiting |
| 2xx | 200-299 | Success | Request succeeded |
| 3xx | 300-399 | Redirection | Additional action needed |
| 4xx | 400-499 | Client Error | Fix request and retry |
| 5xx | 500-599 | Server Error | Server problem |

---

## 11.1 Success 2xx

### 200 OK

Request succeeded.

```
RTSP/1.0 200 OK
CSeq: 2
```

### 250 Low on Storage Space

**RTSP-specific**. Warning for RECORD requests.

```
RTSP/1.0 250 Low on Storage Space
CSeq: 5
Session: 12345678
Range: npt=0-3600
```

Server indicates it may only record up to the specified range.

---

## 11.2 Redirection 3xx

Used for load balancing or redirecting to closer servers.

### 301 Moved Permanently

Resource permanently at new location.

```
RTSP/1.0 301 Moved Permanently
CSeq: 2
Location: rtsp://newserver.example.com/movie
```

### 302 Moved Temporarily

Resource temporarily at new location.

```
RTSP/1.0 302 Moved Temporarily
CSeq: 2
Location: rtsp://backup.example.com/movie
```

### 303 See Other

Use different URI (after POST-like request).

### 304 Not Modified

Resource not modified since If-Modified-Since.

### 305 Use Proxy

Must access through specified proxy.

```
RTSP/1.0 305 Use Proxy
CSeq: 2
Location: rtsp://proxy.example.com:554/
```

---

## 11.3 Client Error 4xx

### Standard HTTP Errors

| Code | Phrase | Description |
|------|--------|-------------|
| 400 | Bad Request | Malformed syntax |
| 401 | Unauthorized | Authentication required |
| 402 | Payment Required | Reserved |
| 403 | Forbidden | Server refuses request |
| 404 | Not Found | Resource doesn't exist |
| 405 | Method Not Allowed | Method not supported |
| 406 | Not Acceptable | Can't satisfy Accept headers |
| 407 | Proxy Authentication Required | Proxy auth needed |
| 408 | Request Timeout | Client took too long |
| 410 | Gone | Resource permanently gone |
| 411 | Length Required | Content-Length required |
| 412 | Precondition Failed | Condition not met |
| 413 | Request Entity Too Large | Body too large |
| 414 | Request-URI Too Long | URI too long |
| 415 | Unsupported Media Type | Content-Type not supported |

### 405 Method Not Allowed

Method not valid for this resource.

```
RTSP/1.0 405 Method Not Allowed
CSeq: 3
Allow: DESCRIBE, SETUP, PLAY, PAUSE, TEARDOWN
```

MUST include `Allow` header listing valid methods.

---

### RTSP-Specific Client Errors (451-462)

### 451 Parameter Not Understood

Server doesn't recognize a parameter.

```
RTSP/1.0 451 Parameter Not Understood
CSeq: 5
```

### 452 Conference Not Found

Unknown conference ID in Conference header.

```
RTSP/1.0 452 Conference Not Found
CSeq: 6
```

### 453 Not Enough Bandwidth

Insufficient bandwidth for request.

```
RTSP/1.0 453 Not Enough Bandwidth
CSeq: 7
```

### 454 Session Not Found

Session ID missing, invalid, or expired.

```
RTSP/1.0 454 Session Not Found
CSeq: 8
```

**Common causes:**
- Session timed out
- Wrong Session header value
- Session ID never existed

### 455 Method Not Valid in This State

Method cannot be used in current state. See [State Machines](A-state-machines.md) for valid state transitions.

```
RTSP/1.0 455 Method Not Valid in This State
CSeq: 9
Allow: SETUP
```

**Example**: PLAY before SETUP.

### 456 Header Field Not Valid for Resource

Header not applicable to resource.

```
RTSP/1.0 456 Header Field Not Valid for Resource
CSeq: 10
```

**Example**: Range header on non-seekable stream.

### 457 Invalid Range

Range value out of bounds.

```
RTSP/1.0 457 Invalid Range
CSeq: 11
```

**Example**: Requesting `npt=1000-` on 30-second clip.

### 458 Parameter Is Read-Only

SET_PARAMETER on read-only value.

```
RTSP/1.0 458 Parameter Is Read-Only
CSeq: 12
```

### 459 Aggregate Operation Not Allowed

Request must use stream URL, not presentation URL.

```
RTSP/1.0 459 Aggregate Operation Not Allowed
CSeq: 13
```

**When**: SETUP with aggregate URL when server requires per-stream SETUP.

### 460 Only Aggregate Operation Allowed

Request must use presentation URL, not stream URL.

```
RTSP/1.0 460 Only Aggregate Operation Allowed
CSeq: 14
```

**When**: PAUSE on individual stream when server requires aggregate control.

### 461 Unsupported Transport

Transport specification not supported. See [Transport Header](10-transport-header.md) for valid formats.

```
RTSP/1.0 461 Unsupported Transport
CSeq: 15
```

### 462 Destination Unreachable

Can't send to specified destination.

```
RTSP/1.0 462 Destination Unreachable
CSeq: 16
```

**Common causes:**
- Client behind NAT
- Firewall blocking
- Invalid destination IP

---

## 11.4 Server Error 5xx

### Standard HTTP Errors

| Code | Phrase | Description |
|------|--------|-------------|
| 500 | Internal Server Error | Generic server failure |
| 501 | Not Implemented | Method not implemented |
| 502 | Bad Gateway | Upstream server error |
| 503 | Service Unavailable | Temporarily overloaded |
| 504 | Gateway Timeout | Upstream timeout |
| 505 | RTSP Version Not Supported | Unsupported version |

### 501 Not Implemented

Server doesn't support the method.

```
RTSP/1.0 501 Not Implemented
CSeq: 5
```

### 551 Option Not Supported

**RTSP-specific**. Required feature not supported.

```
RTSP/1.0 551 Option Not Supported
CSeq: 17
Unsupported: funky-feature
```

Response to `Require: funky-feature` when server doesn't support it.

---

## Quick Reference: RTSP-Specific Codes

| Code | Phrase | Typical Cause |
|------|--------|---------------|
| 250 | Low on Storage Space | Disk running low (RECORD) |
| 451 | Parameter Not Understood | Unknown parameter |
| 452 | Conference Not Found | Invalid Conference header |
| 453 | Not Enough Bandwidth | Bandwidth limit |
| 454 | Session Not Found | Expired/invalid session |
| 455 | Method Not Valid in This State | Wrong state |
| 456 | Header Field Not Valid | Inapplicable header |
| 457 | Invalid Range | Range out of bounds |
| 458 | Parameter Is Read-Only | Can't modify |
| 459 | Aggregate Operation Not Allowed | Need stream URL |
| 460 | Only Aggregate Operation Allowed | Need presentation URL |
| 461 | Unsupported Transport | Transport not supported |
| 462 | Destination Unreachable | Network issue |
| 551 | Option Not Supported | Missing feature |

---

## Error Handling Best Practices

### Client Implementation

```
response = send_request(request)

if response.code >= 500:
    # Server error - retry with backoff
    retry_with_backoff()
elif response.code >= 400:
    # Client error - fix and retry
    if response.code == 454:
        # Session expired - re-SETUP
        session_id = None
        setup_again()
    elif response.code == 461:
        # Try different transport
        try_alternate_transport()
    else:
        # Log error, inform user
        handle_error(response)
elif response.code >= 300:
    # Redirect
    new_url = response.headers['Location']
    redirect_to(new_url)
else:
    # Success
    process_success(response)
```

---

> [Back to Index](00-index.md) | [Previous: Methods](07-methods.md) | [Next: Headers](09-headers.md)
