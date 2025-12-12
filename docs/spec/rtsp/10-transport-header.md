# Section 12.39: Transport Header

> [Back to Index](00-index.md) | [Previous: Headers](09-headers.md) | [Next: Caching](11-caching.md)

## Overview

The Transport header is **critical** for RTSP implementation. It specifies how media data will be delivered.

---

## Transport Header Syntax

```
Transport = "Transport" ":" 1#transport-spec
transport-spec = transport-protocol/profile[/lower-transport] *parameter
```

### Format

```
transport-protocol/profile/lower-transport;param1;param2=value;...
```

### Examples

```
Transport: RTP/AVP;unicast;client_port=4588-4589
Transport: RTP/AVP/UDP;multicast;ttl=127
Transport: RTP/AVP/TCP;unicast;interleaved=0-1
Transport: RTP/AVP;multicast;destination=224.2.0.1;port=3456-3457;ttl=16
```

---

## Transport Specification Components

### Transport Protocol

| Value | Description |
|-------|-------------|
| `RTP` | Real-time Transport Protocol |

### Profile

| Value | Description |
|-------|-------------|
| `AVP` | Audio/Video Profile (RFC 3551) |

### Lower Transport

| Value | Description | Default |
|-------|-------------|---------|
| `UDP` | User Datagram Protocol | Yes (for RTP/AVP) |
| `TCP` | Transmission Control Protocol | No |

---

## General Parameters

### unicast / multicast

Delivery mode. **Mutually exclusive**.

```
Transport: RTP/AVP;unicast;client_port=4588-4589
Transport: RTP/AVP;multicast;destination=224.2.0.1;port=3456-3457;ttl=16
```

| Mode | Default | Description |
|------|---------|-------------|
| `multicast` | Yes | Multiple recipients |
| `unicast` | No | Single recipient |

### destination

Destination address for media.

```
Transport: RTP/AVP;unicast;destination=192.168.1.100;client_port=4588-4589
Transport: RTP/AVP;multicast;destination=224.2.0.1
```

**Security Warning**: Servers SHOULD authenticate clients before allowing custom destinations to prevent reflection attacks.

### source

Source address (if different from RTSP connection).

```
Transport: RTP/AVP;unicast;source=10.0.0.1;client_port=4588-4589
```

### layers

Number of multicast layers.

```
Transport: RTP/AVP;multicast;layers=3;destination=224.2.0.1
```

Streams sent to consecutive addresses starting at destination.

### mode

Methods to support for this session.

```
Transport: RTP/AVP;unicast;client_port=4588-4589;mode="PLAY"
Transport: RTP/AVP;unicast;client_port=4588-4589;mode="RECORD"
```

| Value | Description |
|-------|-------------|
| `PLAY` | Playback (default) |
| `RECORD` | Recording |

### append

Append to existing resource (with RECORD).

```
Transport: RTP/AVP;unicast;client_port=4588-4589;mode="RECORD";append
```

---

## Unicast Parameters

### client_port

RTP/RTCP port pair on client.

```
Transport: RTP/AVP;unicast;client_port=4588-4589
```

- First port: RTP
- Second port: RTCP (typically RTP + 1)

### server_port

RTP/RTCP port pair on server (response only).

```
Transport: RTP/AVP;unicast;client_port=4588-4589;server_port=6256-6257
```

### ssrc

RTP Synchronization Source identifier.

```
Transport: RTP/AVP;unicast;client_port=4588-4589;ssrc=0A3C4D5E
```

- 8 hex characters
- Request: Preferred SSRC
- Response: Actual SSRC

---

## Multicast Parameters

### ttl

Time-to-live for multicast packets.

```
Transport: RTP/AVP;multicast;destination=224.2.0.1;port=3456-3457;ttl=16
```

### port

RTP/RTCP port pair for multicast.

```
Transport: RTP/AVP;multicast;destination=224.2.0.1;port=3456-3457
```

---

## Interleaved (TCP) Parameters

### interleaved

Channel numbers for interleaved data over TCP.

```
Transport: RTP/AVP/TCP;unicast;interleaved=0-1
```

- First number: RTP channel
- Second number: RTCP channel

### Interleaved Data Format

```
+--------+--------+------------------+----------------+
|  '$'   |channel |  length (16-bit) |     data       |
| (0x24) |        |   big-endian     |                |
+--------+--------+------------------+----------------+
```

---

## Transport Negotiation Flow

### Client Request (Multiple Options)

```
SETUP rtsp://server/movie/trackID=1 RTSP/1.0
CSeq: 3
Transport: RTP/AVP;unicast;client_port=4588-4589,
           RTP/AVP/TCP;unicast;interleaved=0-1
```

### Server Response (Single Selection)

```
RTSP/1.0 200 OK
CSeq: 3
Session: 12345678
Transport: RTP/AVP;unicast;client_port=4588-4589;server_port=6256-6257
```

---

## Common Transport Configurations

### UDP Unicast

```
C->S: Transport: RTP/AVP;unicast;client_port=4588-4589

S->C: Transport: RTP/AVP;unicast;client_port=4588-4589;server_port=6256-6257
```

```
Client:4588 <---- RTP ---- Server:6256
Client:4589 <---- RTCP --- Server:6257
```

### UDP Multicast

```
C->S: Transport: RTP/AVP;multicast

S->C: Transport: RTP/AVP;multicast;destination=224.2.0.1;port=3456-3457;ttl=16
```

```
Multicast Group: 224.2.0.1
Client joins group, receives on ports 3456 (RTP), 3457 (RTCP)
```

### TCP Interleaved

```
C->S: Transport: RTP/AVP/TCP;unicast;interleaved=0-1

S->C: Transport: RTP/AVP/TCP;unicast;interleaved=0-1
```

```
RTSP connection carries:
- Channel 0: RTP packets
- Channel 1: RTCP packets
```

---

## Multiple Streams

Each stream needs separate SETUP with unique ports/channels.

### Stream 1

```
SETUP rtsp://server/movie/trackID=1 RTSP/1.0
Transport: RTP/AVP;unicast;client_port=4588-4589
```

### Stream 2

```
SETUP rtsp://server/movie/trackID=2 RTSP/1.0
Transport: RTP/AVP;unicast;client_port=4590-4591
Session: 12345678
```

### Interleaved Multiple Streams

```
Stream 1: Transport: RTP/AVP/TCP;unicast;interleaved=0-1
Stream 2: Transport: RTP/AVP/TCP;unicast;interleaved=2-3
```

---

## Transport Header ABNF

```abnf
Transport        = "Transport" ":" 1#transport-spec
transport-spec   = transport-protocol "/" profile ["/" lower-transport]
                   *parameter
transport-protocol = "RTP"
profile          = "AVP"
lower-transport  = "TCP" / "UDP"
parameter        = ( "unicast" / "multicast" )
                 / ";" "destination" [ "=" address ]
                 / ";" "interleaved" "=" channel [ "-" channel ]
                 / ";" "append"
                 / ";" "ttl" "=" ttl
                 / ";" "layers" "=" 1*DIGIT
                 / ";" "port" "=" port [ "-" port ]
                 / ";" "client_port" "=" port [ "-" port ]
                 / ";" "server_port" "=" port [ "-" port ]
                 / ";" "ssrc" "=" ssrc
                 / ";" "mode" "=" <"> 1#mode <">
ttl              = 1*3DIGIT
port             = 1*5DIGIT
ssrc             = 8HEX
channel          = 1*3DIGIT
address          = host
mode             = "PLAY" / "RECORD"
```

---

## Implementation Checklist

### Client MUST Support

- [ ] Parse Transport header in response
- [ ] Handle `client_port` parameter
- [ ] Handle `server_port` parameter
- [ ] Support at least one transport type

### Client SHOULD Support

- [ ] UDP unicast
- [ ] TCP interleaved (for firewall traversal)
- [ ] Multiple transport options in request

### Server MUST Support

- [ ] Parse Transport header in request
- [ ] Select from offered transports
- [ ] Return single transport in response
- [ ] Return error if no transport acceptable (461)

---

## Error Handling

### 461 Unsupported Transport

```
C->S: SETUP rtsp://server/movie RTSP/1.0
      Transport: RTP/AVP/SCTP;unicast

S->C: RTSP/1.0 461 Unsupported Transport
      CSeq: 3
```

### Transport Change During Session

Server MAY allow transport changes:

```
C->S: SETUP rtsp://server/movie/trackID=1 RTSP/1.0
      CSeq: 10
      Session: 12345678
      Transport: RTP/AVP/TCP;unicast;interleaved=0-1
```

If not allowed:

```
S->C: RTSP/1.0 455 Method Not Valid in This State
      CSeq: 10
```

---

> [Back to Index](00-index.md) | [Previous: Headers](09-headers.md) | [Next: Caching](11-caching.md)
