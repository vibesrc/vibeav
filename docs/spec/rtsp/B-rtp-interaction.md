# Appendix B: Interaction with RTP

> [Back to Index](00-index.md) | [Previous: State Machines](A-state-machines.md) | [Next: SDP Usage](C-sdp-usage.md)

## Overview

RTP (Real-time Transport Protocol) is the standard transport for RTSP media streams. RTSP controls the session while RTP delivers the actual media data.

---

## RTP/RTCP Basics

### RTP

- Carries media payload (audio/video)
- Provides timing information
- Includes sequence numbers for ordering
- Supports multiple payload types

### RTCP

- Companion protocol to RTP
- Provides synchronization (SR/RR)
- Reports quality statistics
- Shares participant information

---

## Timestamp Continuity

### Key Requirement

**RTP sequence numbers and timestamps MUST be continuous across NPT jumps.**

### Why This Matters

Media renderers (RTP layer) should not be affected by seek operations. If timestamps jump, the renderer may:
- Assume a pause occurred
- Believe packets are duplicates
- Lose synchronization

### Example

```
Clock: 8000 Hz
Packetization: 100 ms
Initial seq=0, timestamp=0

Segment 1: NPT 10-15 (5 seconds)
  Packets: seq 0-49, timestamps 0-39,200

[SEEK to NPT 18]

Segment 2: NPT 18-20 (2 seconds)
  Packets: seq 50-69, timestamps 40,000-55,200

Note: seq and timestamp are CONTINUOUS despite NPT gap
```

---

## RTP-Info Header

Provides synchronization information in PLAY response.

### Format

```
RTP-Info: url=<stream-url>;seq=<sequence>;rtptime=<timestamp>
```

### Parameters

| Parameter | Description |
|-----------|-------------|
| `url` | Stream URL |
| `seq` | First RTP sequence number |
| `rtptime` | RTP timestamp of first packet |

### Example

```
RTSP/1.0 200 OK
CSeq: 5
Session: 12345678
Range: npt=10-20
RTP-Info: url=rtsp://server/movie/trackID=1;seq=12345;rtptime=3450012,
          url=rtsp://server/movie/trackID=2;seq=54321;rtptime=2876543
```

---

## Seek Handling

### Client Perspective

1. Send PLAY with new Range
2. Receive RTP-Info in response
3. Use seq/rtptime to identify new packets
4. Continue rendering without gap

### Server Perspective

1. Receive PLAY with Range
2. Calculate new RTP seq/timestamp
3. Send RTP-Info with new values
4. Start streaming from new position

### Identifying New Packets

```
Before seek: Receiving packets with seq around 10000
After seek:  RTP-Info says seq=50000

Client knows packets 10001-49999 don't exist,
packets starting at 50000 are new stream position.
```

---

## Marker Bit Usage

### Audio Streams

Server SHOULD set RTP marker bit:
- At beginning of new PLAY request
- Allows client to perform playout buffer adjustments

### Video Streams

Marker bit typically indicates:
- End of video frame
- Follows normal RTP video conventions

---

## RTCP Synchronization

### Sender Reports (SR)

RTCP SR provides:
- NTP timestamp (wall clock)
- RTP timestamp (media clock)
- Mapping between real time and media time

### Multi-Stream Synchronization

Even with streams from different servers:

```
Audio Server sends RTCP SR:
  NTP: 2024-01-15 10:00:00.000
  RTP timestamp: 44100 (audio at 44.1kHz)

Video Server sends RTCP SR:
  NTP: 2024-01-15 10:00:00.000
  RTP timestamp: 2700000 (video at 90kHz)

Client aligns streams using NTP correlation.
```

---

## Interleaved RTP/RTCP

When using TCP transport, RTP/RTCP packets are embedded in RTSP connection. See [Transport Header](10-transport-header.md) and [Methods - Interleaved Binary Data](07-methods.md#1012-embedded-interleaved-binary-data) for setup details.

### Channel Assignment

```
Transport: RTP/AVP/TCP;unicast;interleaved=0-1
```

- Channel 0: RTP packets
- Channel 1: RTCP packets

### Frame Format

```
+--------+--------+--------+--------+--------+...
|  '$'   |channel |   length (BE)   |  RTP/RTCP data
| (0x24) | (0-1)  |                 |
+--------+--------+--------+--------+--------+...
```

### Parsing Example

```python
def read_interleaved_frame(socket):
    marker = socket.read(1)
    if marker != b'$':
        # Start of RTSP message
        return read_rtsp_message(socket, marker)

    channel = socket.read(1)[0]
    length = struct.unpack('>H', socket.read(2))[0]
    data = socket.read(length)

    if channel % 2 == 0:
        return ("RTP", channel, data)
    else:
        return ("RTCP", channel, data)
```

---

## Scaling and Speed

### Scale Header

Affects playback speed AND timestamp generation:

```
Scale: 2    ; 2x fast forward
Scale: -1   ; Reverse playback
Scale: 0.5  ; Half speed
```

**RTP timestamp progression changes with scale.**

### Speed Header

Affects delivery rate, NOT presentation:

```
Speed: 2.5  ; Deliver 2.5x faster
```

Used for buffer fill, not playback speed.

---

## RTP Profile

### AVP (Audio/Video Profile)

Standard profile defined in RFC 3551.

```
Transport: RTP/AVP;unicast;client_port=5000-5001
```

### Static Payload Types

| PT | Encoding | Clock Rate |
|----|----------|------------|
| 0 | PCMU | 8000 |
| 3 | GSM | 8000 |
| 8 | PCMA | 8000 |
| 14 | MPA | 90000 |
| 26 | JPEG | 90000 |
| 31 | H261 | 90000 |
| 32 | MPV | 90000 |
| 33 | MP2T | 90000 |

### Dynamic Payload Types (96-127)

Defined in SDP:

```
m=video 0 RTP/AVP 96
a=rtpmap:96 H264/90000
```

---

## Implementation Notes

### Client Implementation

1. **Track RTP-Info**: Store seq/rtptime from PLAY response
2. **Handle discontinuity**: Use RTP-Info to detect seek
3. **Use RTCP**: Process SR for sync
4. **Marker bit**: Reset jitter buffer on marker

### Server Implementation

1. **Maintain continuity**: Don't reset seq/timestamp on seek
2. **Generate RTP-Info**: Include in PLAY response
3. **Send RTCP SR**: Provide timing correlation
4. **Set marker bit**: On new PLAY requests

---

## Common Issues

### Timestamp Wrap

RTP timestamps are 32-bit, will wrap around.

```
At 90kHz, wraps every ~13 hours
At 8kHz, wraps every ~6 days
```

Handle wrap in synchronization code.

### SSRC Changes

SSRC may change on:
- Server restart
- Session re-establishment
- Some seek implementations

Check SSRC in RTP-Info if provided.

### Lost Packets

Use sequence numbers to detect gaps:

```
Received seq: 100, 101, 102, 105, 106
Gap detected: 103, 104 missing
```

---

## Integration Summary

```
+------------------+     Control     +------------------+
|   RTSP Client    | <-------------> |   RTSP Server    |
+------------------+                 +------------------+
        |                                    |
        | RTP/RTCP                           | RTP/RTCP
        v                                    v
+------------------+     Media       +------------------+
|   RTP Receiver   | <-------------- |   RTP Sender     |
+------------------+                 +------------------+
        |                                    |
        v                                    v
+------------------+                 +------------------+
|  Media Renderer  |                 |  Media Source    |
+------------------+                 +------------------+
```

---

> [Back to Index](00-index.md) | [Previous: State Machines](A-state-machines.md) | [Next: SDP Usage](C-sdp-usage.md)
