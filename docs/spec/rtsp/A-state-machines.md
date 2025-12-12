# Appendix A: RTSP Protocol State Machines

> [Back to Index](00-index.md) | [Previous: Security](14-security.md) | [Next: RTP Interaction](B-rtp-interaction.md)

## Overview

State is maintained per (stream URL, session ID) pair. Aggregate URL operations affect all streams in the presentation.

---

## State Definitions

| State | Description |
|-------|-------------|
| **Init** | Before successful SETUP |
| **Ready** | SETUP complete or PAUSE received |
| **Playing** | PLAY successful, media flowing |
| **Recording** | RECORD successful, receiving media |

---

## A.1 Client State Machine

### State Diagram

```
                 +------+
                 | Init |
                 +------+
                    |
                    | SETUP (success)
                    v
    +------+     +-------+     +---------+
    | Init | <-- | Ready | --> | Playing |
    +------+     +-------+     +---------+
       ^            |   ^          |
       |            |   |          |
       |  TEARDOWN  |   +----------+
       +------------+      PAUSE
                    |
                    | RECORD
                    v
               +-----------+
               | Recording |
               +-----------+
```

### Client State Transitions

| Current State | Message Sent | Next State (on 2xx) |
|---------------|--------------|---------------------|
| Init | SETUP | Ready |
| Init | TEARDOWN | Init |
| Ready | PLAY | Playing |
| Ready | RECORD | Recording |
| Ready | TEARDOWN | Init |
| Ready | SETUP | Ready |
| Playing | PAUSE | Ready |
| Playing | TEARDOWN | Init |
| Playing | PLAY | Playing |
| Playing | SETUP | Playing* |
| Recording | PAUSE | Ready |
| Recording | TEARDOWN | Init |
| Recording | RECORD | Recording |
| Recording | SETUP | Recording* |

\* Transport parameter change

### Response Code Effects

| Response Class | Effect |
|----------------|--------|
| 2xx | Transition to "next state" |
| 3xx | State becomes Init |
| 4xx | No state change |
| 5xx | No state change |

### Implicit Transitions

- **End of range reached**: Playing → Ready
- **REDIRECT received**: Equivalent to 3xx

---

## A.2 Server State Machine

### State Diagram

```
                 +------+
                 | Init |
                 +------+
                    |
                    | SETUP received (success)
                    v
    +------+     +-------+     +---------+
    | Init | <-- | Ready | --> | Playing |
    +------+     +-------+     +---------+
       ^            |   ^          |
       |  TEARDOWN  |   | PAUSE    |
       +------------+   +----------+
       |            |
       |            | RECORD
       |            v
       |       +-----------+
       +------ | Recording |
         TEARDOWN +-----------+
```

### Server State Transitions

| Current State | Message Received | Next State |
|---------------|------------------|------------|
| Init | SETUP | Ready |
| Init | TEARDOWN | Init |
| Ready | PLAY | Playing |
| Ready | SETUP | Ready |
| Ready | TEARDOWN | Init |
| Ready | RECORD | Recording |
| Playing | PLAY | Playing |
| Playing | PAUSE | Ready |
| Playing | TEARDOWN | Init |
| Playing | SETUP | Playing |
| Recording | RECORD | Recording |
| Recording | PAUSE | Ready |
| Recording | TEARDOWN | Init |
| Recording | SETUP | Recording |

### Timeout Transitions

- **Playing/Recording (unicast)**: No activity for timeout → Init
- **Ready**: No activity for timeout → Init

Default timeout: 60 seconds (configurable via Session header)

---

## State-Independent Methods

These methods do NOT affect state. See [Methods](07-methods.md) for details:

| Method | Description |
|--------|-------------|
| [OPTIONS](07-methods.md#101-options) | Query capabilities |
| [DESCRIBE](07-methods.md#102-describe) | Get presentation description |
| [ANNOUNCE](07-methods.md#103-announce) | Post/update description |
| [GET_PARAMETER](07-methods.md#108-get_parameter) | Retrieve parameter |
| [SET_PARAMETER](07-methods.md#109-set_parameter) | Set parameter |

---

## Aggregate Control State

With aggregate control, state applies to all streams:

### Example: Two Streams

```
Presentation: rtsp://server/movie
  Stream 1: rtsp://server/movie/audio
  Stream 2: rtsp://server/movie/video
```

```
SETUP rtsp://server/movie/audio   → audio: Ready, video: Init
SETUP rtsp://server/movie/video   → audio: Ready, video: Ready
PLAY rtsp://server/movie          → audio: Playing, video: Playing
PAUSE rtsp://server/movie         → audio: Ready, video: Ready
TEARDOWN rtsp://server/movie      → audio: Init, video: Init
```

---

## Invalid Operations

Attempting invalid state transitions returns error:

### 455 Method Not Valid in This State

```
# Client in Init state
PLAY rtsp://server/movie RTSP/1.0
CSeq: 1
Session: 12345678

RTSP/1.0 455 Method Not Valid in This State
CSeq: 1
Allow: SETUP
```

### Common Invalid Transitions

| State | Invalid Method |
|-------|----------------|
| Init | PLAY, PAUSE, RECORD |
| Ready | PAUSE (no effect) |
| Playing | RECORD |
| Recording | PLAY |

---

## Implementation Example

### Client State Machine (Pseudocode)

```python
class RTSPClientState:
    INIT = "init"
    READY = "ready"
    PLAYING = "playing"
    RECORDING = "recording"

class RTSPClient:
    def __init__(self):
        self.state = RTSPClientState.INIT
        self.session_id = None

    def handle_response(self, method, status_code):
        if status_code >= 300 and status_code < 400:
            # Redirect
            self.state = RTSPClientState.INIT
            self.session_id = None
            return

        if status_code >= 400:
            # Error - no state change
            return

        # Success (2xx)
        if method == "SETUP":
            self.state = RTSPClientState.READY
        elif method == "PLAY":
            self.state = RTSPClientState.PLAYING
        elif method == "RECORD":
            self.state = RTSPClientState.RECORDING
        elif method == "PAUSE":
            self.state = RTSPClientState.READY
        elif method == "TEARDOWN":
            self.state = RTSPClientState.INIT
            self.session_id = None

    def can_send(self, method):
        valid_methods = {
            RTSPClientState.INIT: ["SETUP", "TEARDOWN", "OPTIONS", "DESCRIBE"],
            RTSPClientState.READY: ["PLAY", "RECORD", "SETUP", "TEARDOWN", "OPTIONS"],
            RTSPClientState.PLAYING: ["PAUSE", "PLAY", "SETUP", "TEARDOWN", "OPTIONS"],
            RTSPClientState.RECORDING: ["PAUSE", "RECORD", "SETUP", "TEARDOWN", "OPTIONS"],
        }
        return method in valid_methods.get(self.state, [])
```

### Server State Machine (Pseudocode)

```python
class RTSPServerState:
    INIT = "init"
    READY = "ready"
    PLAYING = "playing"
    RECORDING = "recording"

class RTSPSession:
    def __init__(self, session_id):
        self.session_id = session_id
        self.state = RTSPServerState.INIT
        self.last_activity = time.time()
        self.timeout = 60

    def handle_request(self, method):
        self.last_activity = time.time()

        transitions = {
            (RTSPServerState.INIT, "SETUP"): RTSPServerState.READY,
            (RTSPServerState.READY, "PLAY"): RTSPServerState.PLAYING,
            (RTSPServerState.READY, "RECORD"): RTSPServerState.RECORDING,
            (RTSPServerState.PLAYING, "PAUSE"): RTSPServerState.READY,
            (RTSPServerState.RECORDING, "PAUSE"): RTSPServerState.READY,
        }

        key = (self.state, method)
        if key in transitions:
            self.state = transitions[key]
            return 200
        elif method == "TEARDOWN":
            self.state = RTSPServerState.INIT
            return 200
        elif method in ["OPTIONS", "DESCRIBE", "GET_PARAMETER", "SET_PARAMETER"]:
            return 200
        else:
            return 455  # Method Not Valid in This State

    def is_expired(self):
        return time.time() - self.last_activity > self.timeout
```

---

## State Machine Summary

### Client

```
           SETUP
   Init ─────────> Ready
    ^               │ │
    │               │ │ PLAY
    │    TEARDOWN   │ ├────────> Playing
    │<──────────────┤ │           │
    │               │ │ RECORD    │ PAUSE
    │               │ └─────────> Recording
    │               │             │
    │<──────────────┴─────────────┘
              TEARDOWN
```

### Server

Same transitions, triggered by received messages instead of sent messages.

---

> [Back to Index](00-index.md) | [Previous: Security](14-security.md) | [Next: RTP Interaction](B-rtp-interaction.md)
