# Section 13: Caching

> [Back to Index](00-index.md) | [Previous: Transport Header](10-transport-header.md) | [Next: Examples](12-examples.md)

## Overview

RTSP caching differs from HTTP because RTSP sessions are stateful and media data is delivered out-of-band.

---

## Cacheable Elements

| Element | Cacheable | Notes |
|---------|-----------|-------|
| Presentation description | Yes | SDP from DESCRIBE |
| Media data | Sometimes | Via RTP, not RTSP |
| Session state | No | Dynamic, stateful |
| SETUP responses | No | Per-client state |

---

## Cache-Control Header

Controls caching behavior for session descriptions.

### Directives

| Directive | Meaning |
|-----------|---------|
| `no-cache` | Must revalidate before use |
| `public` | Any cache may store |
| `private` | Only client cache |
| `no-transform` | Don't modify content |
| `only-if-cached` | Only return if cached |
| `max-stale[=n]` | Accept stale up to n seconds |
| `min-fresh=n` | Need n seconds freshness |
| `must-revalidate` | MUST revalidate when stale |
| `proxy-revalidate` | Proxies MUST revalidate |

### Example

```
Cache-Control: public, max-age=3600
```

Description cacheable for 1 hour.

---

## Expires Header

Specifies when content becomes stale.

```
Expires: Thu, 01 Dec 1994 16:00:00 GMT
```

### Special Values

- Past date: Already expired, don't cache
- Date in far future: Cache indefinitely

---

## Session Description Caching

### Validation

Use `If-Modified-Since` to check freshness:

```
DESCRIBE rtsp://server/movie RTSP/1.0
CSeq: 2
Accept: application/sdp
If-Modified-Since: Sat, 29 Oct 1994 19:43:31 GMT
```

If unchanged:
```
RTSP/1.0 304 Not Modified
CSeq: 2
```

### ETag-Based Validation

SDP can include entity tag:
```
a=etag:158bb3e7c7fd62ce67f12b533f06b83a
```

Use `If-Match` in SETUP:
```
SETUP rtsp://server/movie/trackID=1 RTSP/1.0
CSeq: 3
If-Match: "158bb3e7c7fd62ce67f12b533f06b83a"
Transport: RTP/AVP;unicast;client_port=4588-4589
```

If description changed:
```
RTSP/1.0 412 Precondition Failed
CSeq: 3
```

---

## Proxy Caching Considerations

### Stateful Sessions

RTSP sessions maintain state:
- Proxies cannot cache SETUP, PLAY, PAUSE, TEARDOWN
- Each client needs own session

### Description Caching

Proxies CAN cache DESCRIBE responses:
- Same URL → same description (usually)
- Check `Cache-Control` and `Expires`

### Stream Data Caching

For multicast:
- Multiple clients can share one stream
- Proxy joins multicast, redistributes

For unicast:
- Each client needs separate stream
- Proxy may optimize by sharing server connection

---

## Best Practices

### Server Implementation

1. **Include Cache-Control** in DESCRIBE responses
2. **Set appropriate Expires** for dynamic content
3. **Use ETag** for version tracking
4. **Return 304** for unmodified content

### Client Implementation

1. **Cache descriptions** based on headers
2. **Validate before SETUP** if cached
3. **Handle 304 and 412** responses
4. **Respect Cache-Control** directives

### Example Flow

```
# First request
DESCRIBE rtsp://server/movie RTSP/1.0
CSeq: 1

RTSP/1.0 200 OK
CSeq: 1
Cache-Control: public, max-age=600
Content-Type: application/sdp
Content-Length: 376

v=0
...
a=etag:abc123
...

# Later request (within 600 seconds)
[Use cached description]

# After expiry
DESCRIBE rtsp://server/movie RTSP/1.0
CSeq: 2
If-Modified-Since: [cached date]

RTSP/1.0 304 Not Modified
CSeq: 2
[Use cached description]
```

---

## Caching Summary

| What | Can Cache | How Long |
|------|-----------|----------|
| DESCRIBE response | Yes | Per Cache-Control/Expires |
| SDP content | Yes | Validate with If-Match/If-Modified-Since |
| SETUP response | No | Session-specific |
| PLAY/PAUSE response | No | Transient |
| RTP stream | Maybe | Application-dependent |

---

> [Back to Index](00-index.md) | [Previous: Transport Header](10-transport-header.md) | [Next: Examples](12-examples.md)
