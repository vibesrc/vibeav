# Sections 4-5: RTSP Message Format & General Headers

> [Back to Index](00-index.md) | [Previous: Protocol Parameters](03-protocol-parameters.md) | [Next: Request/Response](05-request-response.md)

## 4. RTSP Message Structure

RTSP messages follow HTTP/1.1 message structure. For full ABNF syntax, see [Syntax](13-syntax.md).

### Generic Message Format

```
RTSP-message = Request | Response

generic-message = start-line
                  *(message-header CRLF)
                  CRLF
                  [message-body]
```

### Components

| Component | Description |
|-----------|-------------|
| start-line | Request-Line or Status-Line |
| message-header | Header field name-value pairs |
| CRLF | Blank line separating headers from body |
| message-body | Optional payload (e.g., SDP) |

---

## 4.1 Message Types

### Request

```
Request = Request-Line
          *(general-header | request-header | entity-header) CRLF
          CRLF
          [message-body]
```

### Response

```
Response = Status-Line
           *(general-header | response-header | entity-header) CRLF
           CRLF
           [message-body]
```

---

## 4.2 Message Headers

Headers are key-value pairs:

```
message-header = field-name ":" [field-value]
field-name     = token
field-value    = *( field-content | LWS )
```

### Header Categories

| Category | Scope | Examples |
|----------|-------|----------|
| General | Request & Response | CSeq, Connection, Date |
| Request | Request only | Accept, Transport, Session |
| Response | Response only | Public, Server, Unsupported |
| Entity | Message body | Content-Type, Content-Length |

### Header Continuation

Long header values can be continued:

```
Transport: RTP/AVP;unicast;
           client_port=4588-4589
```

---

## 4.3 Message Body

The message body contains the entity-body (if any).

### Common Body Types

| Content-Type | Use | See Also |
|--------------|-----|----------|
| `application/sdp` | Session description | [SDP Usage](C-sdp-usage.md) |
| `text/parameters` | Parameter data | [GET/SET_PARAMETER](07-methods.md#108-get_parameter) |
| `text/plain` | Text content | - |

---

## 4.4 Message Length

### Determining Message Length

1. **Response with no body** (status 1xx, 204, 304): No message-body
2. **HEAD request response**: No message-body
3. **Content-Length header**: Specifies exact length
4. **No Content-Length**: Read until connection closes (TCP only)

### Content-Length

```
Content-Length: 376
```

**Critical**: MUST be present if body exists and connection is persistent.

---

## 5. General Header Fields

General headers apply to BOTH requests and responses.

### Required General Headers

| Header | Description | Example |
|--------|-------------|---------|
| `CSeq` | Sequence number (REQUIRED) | `CSeq: 312` |

### Optional General Headers

See [Headers](09-headers.md) for complete header reference.

| Header | Description | Example |
|--------|-------------|---------|
| `Cache-Control` | Caching directives | `Cache-Control: no-cache` |
| `Connection` | Connection options | `Connection: close` |
| `Date` | Message date/time | `Date: Sat, 23 Nov 1996 17:05:33 GMT` |
| `Via` | Proxy chain info | `Via: 1.0 proxy.example.com` |

---

## CSeq Header (Critical)

Every RTSP request MUST have a CSeq header.

### Format

```
CSeq = "CSeq" ":" 1*DIGIT
```

### Rules

1. **Starts at any value** (typically 1)
2. **Increments by 1** for each new request
3. **Response echoes request CSeq**
4. **Per-connection sequence** (not per-session)

### Example

```
Client -> Server:
  DESCRIBE rtsp://server/movie RTSP/1.0
  CSeq: 1

Server -> Client:
  RTSP/1.0 200 OK
  CSeq: 1
  Content-Type: application/sdp
  ...

Client -> Server:
  SETUP rtsp://server/movie/audio RTSP/1.0
  CSeq: 2
  Transport: RTP/AVP;unicast;client_port=4588-4589

Server -> Client:
  RTSP/1.0 200 OK
  CSeq: 2
  Session: 12345678
  ...
```

---

## Connection Header

Controls connection behavior.

### Values

| Value | Meaning |
|-------|---------|
| `close` | Close after response |
| `Keep-Alive` | Keep connection open (default) |

### Example

```
Connection: close
```

---

## Date Header

GMT timestamp of message generation.

### Format

```
Date: Sat, 23 Nov 1996 17:05:33 GMT
```

Servers SHOULD include Date in all responses.

---

## Message Structure Summary

```
+------------------------------------------+
|           Start Line                      |
|  (Request-Line or Status-Line)           |
+------------------------------------------+
|           Headers                         |
|  CSeq: 1                                 |
|  Session: 12345678                       |
|  Content-Type: application/sdp           |
|  Content-Length: 376                     |
+------------------------------------------+
|           CRLF (blank line)              |
+------------------------------------------+
|           Message Body                    |
|  v=0                                     |
|  o=- 12345 12345 IN IP4 127.0.0.1       |
|  s=Session                               |
|  ...                                     |
+------------------------------------------+
```

---

> [Back to Index](00-index.md) | [Previous: Protocol Parameters](03-protocol-parameters.md) | [Next: Request/Response](05-request-response.md)
