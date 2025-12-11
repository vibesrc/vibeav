# Section 1: Introduction

> [Back to Index](00-index.md) | [Next: Notation](02-notation.md)

## 1.1 Purpose

RTSP (Real-Time Streaming Protocol) establishes and controls time-synchronized streams of continuous media (audio/video). It does NOT typically deliver the streams itself - it acts as a **"network remote control"** for multimedia servers.

### Key Characteristics

| Feature | Description |
|---------|-------------|
| Session-based | Server maintains session state (unlike stateless HTTP) |
| Bidirectional | Both client AND server can issue requests |
| Transport-agnostic | Works over TCP or UDP |
| Out-of-band data | Media delivered separately (typically via RTP) |
| HTTP-like syntax | Similar request/response structure to HTTP/1.1 |

### Differences from HTTP

| Aspect | HTTP | RTSP |
|--------|------|------|
| State | Stateless | Stateful ([session-based](A-state-machines.md)) |
| Request direction | Client → Server only | Both directions |
| Data delivery | In-band | Out-of-band (typically [RTP](B-rtp-interaction.md)) |
| Encoding | ISO 8859-1 | UTF-8 (ISO 10646) |
| Request-URI | Relative path | Always [absolute URI](03-protocol-parameters.md#32-rtsp-url) |
| Protocol ID | `HTTP/1.1` | `RTSP/1.0` |

---

## 1.2 Requirements

The key words "MUST", "SHALL", "SHOULD", "MAY" follow RFC 2119 definitions.

---

## 1.3 Terminology

### Core Concepts

| Term | Definition |
|------|------------|
| **Aggregate control** | Control of multiple streams using a single timeline |
| **Conference** | Multiparty multimedia presentation |
| **Client** | Issues RTSP requests |
| **Connection** | Transport layer virtual circuit (TCP) |
| **Container file** | File containing multiple media streams |
| **Continuous media** | Data with timing relationship between source and sink |
| **Entity** | Data transferred as message payload |
| **Media initialization** | Datatype/codec-specific setup (out of RTSP scope) |

### Stream Types

| Term | Definition |
|------|------------|
| **Media server** | Server providing playback/recording of media |
| **Media server indirection** | Redirecting client to different server |
| **Presentation** | Set of streams presented to client as complete media feed |
| **Presentation description** | Contains information about streams (e.g., SDP) |
| **Stream** | Single media instance (audio track, video track, etc.) |

### Session Terms

| Term | Definition |
|------|------------|
| **Message** | Basic unit of RTSP communication |
| **Participant** | Member of a conference (may be media server) |
| **Response** | RTSP response message |
| **Request** | RTSP request message |
| **RTSP session** | Full RTSP transaction (SETUP through TEARDOWN) |
| **Transport initialization** | Negotiation of transport parameters (in RTSP) |

---

## 1.4 Protocol Properties

### Key Design Properties

1. **Extensible** - New methods/parameters can be added
2. **Easy to parse** - HTTP-like text-based format
3. **Secure** - Reuses web security mechanisms
4. **Transport-independent** - Works over any reliable transport
5. **Multi-server capable** - Presentation can span multiple servers
6. **Proxy/firewall friendly** - Similar to HTTP in this regard
7. **Separation of concerns** - Stream control separate from conference control

### Protocol Stack Position

```
+------------------+
|   Application    |
+------------------+
|      RTSP        |  ← Control protocol
+------------------+
|   TCP or UDP     |  ← Control transport
+------------------+
|       IP         |
+------------------+

+------------------+
|      Media       |
+------------------+
|       RTP        |  ← Media transport (separate)
+------------------+
|       UDP        |
+------------------+
|       IP         |
+------------------+
```

### Default Ports

See [Protocol Parameters](03-protocol-parameters.md#32-rtsp-url) for URL format.

| Transport | Port |
|-----------|------|
| RTSP/TCP | 554 |
| RTSP/UDP | 554 |

---

## 1.5 Extending RTSP

Extensions are made by:
1. Adding new **methods**
2. Adding new **headers**
3. Adding new **parameters** to existing headers

### Feature Tags

Use `Require` header to mandate features:
```
Require: feature-tag
```

Use `Proxy-Require` for proxy features:
```
Proxy-Require: feature-tag
```

If unsupported, server returns `551 Option not supported` with `Unsupported` header.

---

## 1.6 Overall Operation

### Basic Flow

```
Client                                    Server
  |                                         |
  |  DESCRIBE rtsp://server/media --------> |
  |  <-------- 200 OK (SDP description)     |
  |                                         |
  |  SETUP rtsp://server/media/audio -----> |
  |  <-------- 200 OK (Session: xyz)        |
  |                                         |
  |  SETUP rtsp://server/media/video -----> |
  |  <-------- 200 OK (Session: xyz)        |
  |                                         |
  |  PLAY rtsp://server/media ------------> |
  |  <-------- 200 OK                       |
  |                                         |
  |  <======== RTP media streams =========> |
  |                                         |
  |  TEARDOWN rtsp://server/media --------> |
  |  <-------- 200 OK                       |
```

---

## 1.7 RTSP States

RTSP session progresses through states:

```
        SETUP
          |
          v
+------+ SETUP  +-------+
| Init | -----> | Ready |
+------+        +-------+
                 |     ^
            PLAY |     | PAUSE
                 v     |
              +---------+
              | Playing |
              +---------+
                   |
              TEARDOWN
                   |
                   v
               (closed)
```

### State Descriptions

| State | Description |
|-------|-------------|
| **Init** | Before any SETUP |
| **Ready** | After SETUP or PAUSE; transport established |
| **Playing** | After PLAY; media being delivered |
| **Recording** | After RECORD; media being received |

See [Appendix A](A-state-machines.md) for detailed state machines.

---

## 1.8 Relationship with Other Protocols

### HTTP

- RTSP reuses HTTP concepts (messages, headers, status codes)
- May share same TCP connection
- Uses HTTP authentication mechanisms

### RTP/RTCP

- RTP delivers the actual media data
- RTCP provides synchronization and QoS feedback
- RTSP provides out-of-band control

### SDP

- Session Description Protocol describes presentations
- Returned in DESCRIBE responses
- Contains codec info, transport parameters, stream URLs

See [Appendix C](C-sdp-usage.md) for SDP usage details.

---

> [Back to Index](00-index.md) | [Next: Notation](02-notation.md)
