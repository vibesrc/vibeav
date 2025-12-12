# Appendix D: Minimal RTSP Implementation

> [Back to Index](00-index.md) | [Previous: SDP Usage](C-sdp-usage.md)

## Overview

This appendix defines minimum requirements for interoperable RTSP clients and servers.

**Related sections:**
- [Methods](07-methods.md) - Full method specifications
- [Headers](09-headers.md) - Header field details
- [Transport Header](10-transport-header.md) - Transport negotiation
- [State Machines](A-state-machines.md) - State transitions
- [Examples](12-examples.md) - Complete protocol flows

---

## D.1 Client Requirements

### D.1.1 Basic Playback Client

#### Required Methods (Generate)

| Method | Purpose |
|--------|---------|
| SETUP | Establish transport |
| TEARDOWN | End session |
| PLAY | Start playback |

#### Required Headers (Generate)

| Header | Usage |
|--------|-------|
| CSeq | Every request |
| Session | After SETUP |
| Transport | In SETUP |
| Connection | When needed |

#### Required Headers (Parse)

| Header | Response Type |
|--------|---------------|
| CSeq | All responses |
| Session | SETUP response |
| Transport | SETUP response |
| Connection | All responses |
| Content-Type | Entity responses |
| Content-Length | Entity responses |
| Content-Language | Entity responses |
| Content-Encoding | Entity responses |
| RTP-Info | PLAY response |

#### Required Behaviors

- Understand status code classes (1xx, 2xx, 3xx, 4xx, 5xx)
- Handle session timeouts
- Support at least one transport (UDP or TCP)

#### Recommended

- Support DESCRIBE method
- Support PAUSE method
- Accept SDP from command line/stdin

---

### D.1.2 Authentication-Enabled Client

Additional requirements:

| Component | Requirement |
|-----------|-------------|
| Methods | All from D.1.1 |
| Parse | WWW-Authenticate |
| Generate | Authorization |
| Support | Basic AND Digest authentication |

---

## D.2 Server Requirements

### D.2.1 Basic Playback Server

#### Required Methods (Process)

| Method | Purpose |
|--------|---------|
| SETUP | Accept transport |
| TEARDOWN | Clean up |
| PLAY | Start delivery |

#### Required Headers (Parse)

| Header | Request Type |
|--------|--------------|
| CSeq | All requests |
| Session | After SETUP |
| Transport | SETUP request |

#### Required Headers (Generate)

| Header | Usage |
|--------|-------|
| CSeq | All responses |
| Session | SETUP response |
| Transport | SETUP response |
| Content-Type | With body |
| Content-Length | With body |
| RTP-Info | PLAY response |

#### Required Behaviors

- Generate cryptographically random session IDs
- Implement session timeouts
- Support at least one transport
- Return appropriate error codes
- Send 501 for unimplemented methods

#### Recommended

- Support DESCRIBE method
- Support PAUSE method
- Support OPTIONS method
- Return SDP for DESCRIBE

---

### D.2.2 Authentication-Enabled Server

Additional requirements:

| Component | Requirement |
|-----------|-------------|
| Generate | WWW-Authenticate |
| Parse | Authorization |
| Support | Basic AND/OR Digest |

---

## Minimal Playback Flow

### Client Perspective

```
1. (Optional) DESCRIBE → Get SDP
2. SETUP → Establish transport, get Session ID
3. PLAY → Start receiving media
4. ... receive RTP ...
5. TEARDOWN → End session
```

### Minimum Request Examples

#### SETUP

```
SETUP rtsp://server/stream RTSP/1.0
CSeq: 1
Transport: RTP/AVP;unicast;client_port=5000-5001
```

#### PLAY

```
PLAY rtsp://server/stream RTSP/1.0
CSeq: 2
Session: ABC123
```

#### TEARDOWN

```
TEARDOWN rtsp://server/stream RTSP/1.0
CSeq: 3
Session: ABC123
```

---

## Minimal Recording Client

### Additional Required Methods

| Method | Purpose |
|--------|---------|
| ANNOUNCE | Post description |
| RECORD | Start recording |

### Flow

```
1. ANNOUNCE → Send SDP describing media
2. SETUP → Establish transport for upload
3. RECORD → Start sending media
4. ... send RTP ...
5. TEARDOWN → End session
```

---

## Implementation Checklist

### Minimal Playback Client

- [ ] Generate SETUP request
- [ ] Parse SETUP response (Session, Transport)
- [ ] Generate PLAY request with Session
- [ ] Parse PLAY response (RTP-Info)
- [ ] Handle RTP reception
- [ ] Generate TEARDOWN request
- [ ] Handle all status code classes
- [ ] Maintain CSeq counter
- [ ] Support at least UDP or TCP/interleaved

### Minimal Playback Server

- [ ] Parse SETUP request
- [ ] Generate Session ID
- [ ] Return SETUP response with Transport
- [ ] Parse PLAY request
- [ ] Return PLAY response with RTP-Info
- [ ] Start RTP transmission
- [ ] Parse TEARDOWN request
- [ ] Clean up resources
- [ ] Implement session timeout
- [ ] Return 501 for unsupported methods

---

## Minimal Code Structure

### Client (Pseudocode)

```python
class MinimalRTSPClient:
    def __init__(self, url):
        self.url = url
        self.cseq = 0
        self.session_id = None
        self.socket = None

    def connect(self):
        self.socket = tcp_connect(parse_host(self.url), 554)

    def setup(self, client_ports):
        self.cseq += 1
        request = f"""SETUP {self.url} RTSP/1.0\r
CSeq: {self.cseq}\r
Transport: RTP/AVP;unicast;client_port={client_ports[0]}-{client_ports[1]}\r
\r
"""
        self.socket.send(request)
        response = self.read_response()

        if response.status == 200:
            self.session_id = response.headers['Session'].split(';')[0]
            return response.headers['Transport']
        else:
            raise RTSPError(response.status)

    def play(self, range=None):
        self.cseq += 1
        request = f"""PLAY {self.url} RTSP/1.0\r
CSeq: {self.cseq}\r
Session: {self.session_id}\r
"""
        if range:
            request += f"Range: {range}\r\n"
        request += "\r\n"

        self.socket.send(request)
        response = self.read_response()

        if response.status == 200:
            return response.headers.get('RTP-Info')
        else:
            raise RTSPError(response.status)

    def teardown(self):
        self.cseq += 1
        request = f"""TEARDOWN {self.url} RTSP/1.0\r
CSeq: {self.cseq}\r
Session: {self.session_id}\r
\r
"""
        self.socket.send(request)
        response = self.read_response()
        self.session_id = None
        return response.status == 200
```

### Server (Pseudocode)

```python
class MinimalRTSPServer:
    def __init__(self):
        self.sessions = {}

    def handle_request(self, request):
        method = request.method

        if method == 'SETUP':
            return self.handle_setup(request)
        elif method == 'PLAY':
            return self.handle_play(request)
        elif method == 'TEARDOWN':
            return self.handle_teardown(request)
        else:
            return Response(501, 'Not Implemented')

    def handle_setup(self, request):
        session_id = generate_session_id()
        transport = parse_transport(request.headers['Transport'])

        # Allocate server ports
        server_ports = allocate_ports()

        session = Session(session_id, transport, server_ports)
        self.sessions[session_id] = session

        response_transport = format_transport(
            transport,
            server_ports
        )

        return Response(200, 'OK', {
            'CSeq': request.headers['CSeq'],
            'Session': f'{session_id};timeout=60',
            'Transport': response_transport
        })

    def handle_play(self, request):
        session_id = request.headers['Session']
        session = self.sessions.get(session_id)

        if not session:
            return Response(454, 'Session Not Found')

        # Start RTP streaming
        rtp_info = session.start_streaming()

        return Response(200, 'OK', {
            'CSeq': request.headers['CSeq'],
            'Session': session_id,
            'RTP-Info': rtp_info
        })

    def handle_teardown(self, request):
        session_id = request.headers['Session']
        session = self.sessions.pop(session_id, None)

        if session:
            session.cleanup()

        return Response(200, 'OK', {
            'CSeq': request.headers['CSeq']
        })
```

---

## Error Handling Matrix

| Situation | Response |
|-----------|----------|
| Unknown method | 501 Not Implemented |
| Missing Session | 454 Session Not Found |
| Bad Transport | 461 Unsupported Transport |
| PLAY before SETUP | 455 Method Not Valid in This State |
| Malformed request | 400 Bad Request |
| Auth required | 401 Unauthorized |

---

## Summary

### Absolute Minimum Client

```
SETUP → PLAY → TEARDOWN
```

With proper CSeq, Session, Transport headers.

### Absolute Minimum Server

Handle SETUP, PLAY, TEARDOWN.
Generate Session ID, negotiate Transport, send RTP.

---

> [Back to Index](00-index.md) | [Previous: SDP Usage](C-sdp-usage.md)
