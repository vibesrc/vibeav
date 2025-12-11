# Section 14: Protocol Examples

> [Back to Index](00-index.md) | [Previous: Caching](11-caching.md) | [Next: Syntax](13-syntax.md)

These examples demonstrate complete RTSP interactions. For method details, see [Methods](07-methods.md). For transport options, see [Transport Header](10-transport-header.md).

## 14.1 Media on Demand (Unicast)

Client plays movie from separate audio/video servers.

### Presentation Description (via HTTP)

```
C->W: GET /twister.sdp HTTP/1.1
      Host: www.example.com
      Accept: application/sdp

W->C: HTTP/1.0 200 OK
      Content-Type: application/sdp

      v=0
      o=- 2890844526 2890842807 IN IP4 192.16.24.202
      s=RTSP Session
      m=audio 0 RTP/AVP 0
      a=control:rtsp://audio.example.com/twister/audio.en
      m=video 0 RTP/AVP 31
      a=control:rtsp://video.example.com/twister/video
```

### Setup Audio Stream

```
C->A: SETUP rtsp://audio.example.com/twister/audio.en RTSP/1.0
      CSeq: 1
      Transport: RTP/AVP/UDP;unicast;client_port=3056-3057

A->C: RTSP/1.0 200 OK
      CSeq: 1
      Session: 12345678
      Transport: RTP/AVP/UDP;unicast;client_port=3056-3057;
                 server_port=5000-5001
```

### Setup Video Stream

```
C->V: SETUP rtsp://video.example.com/twister/video RTSP/1.0
      CSeq: 1
      Transport: RTP/AVP/UDP;unicast;client_port=3058-3059

V->C: RTSP/1.0 200 OK
      CSeq: 1
      Session: 23456789
      Transport: RTP/AVP/UDP;unicast;client_port=3058-3059;
                 server_port=5002-5003
```

### Play Both Streams (Starting at 10 minutes)

```
C->V: PLAY rtsp://video.example.com/twister/video RTSP/1.0
      CSeq: 2
      Session: 23456789
      Range: smpte=0:10:00-

V->C: RTSP/1.0 200 OK
      CSeq: 2
      Session: 23456789
      Range: smpte=0:10:00-0:20:00
      RTP-Info: url=rtsp://video.example.com/twister/video;
                seq=12312232;rtptime=78712811

C->A: PLAY rtsp://audio.example.com/twister/audio.en RTSP/1.0
      CSeq: 2
      Session: 12345678
      Range: smpte=0:10:00-

A->C: RTSP/1.0 200 OK
      CSeq: 2
      Session: 12345678
      Range: smpte=0:10:00-0:20:00
      RTP-Info: url=rtsp://audio.example.com/twister/audio.en;
                seq=876655;rtptime=1032181
```

### Teardown

```
C->A: TEARDOWN rtsp://audio.example.com/twister/audio.en RTSP/1.0
      CSeq: 3
      Session: 12345678

A->C: RTSP/1.0 200 OK
      CSeq: 3

C->V: TEARDOWN rtsp://video.example.com/twister/video RTSP/1.0
      CSeq: 3
      Session: 23456789

V->C: RTSP/1.0 200 OK
      CSeq: 3
```

---

## 14.2 Container File with Aggregate Control

Single server with audio+video, aggregate control URL.

### DESCRIBE

```
C->M: DESCRIBE rtsp://foo/twister RTSP/1.0
      CSeq: 1

M->C: RTSP/1.0 200 OK
      CSeq: 1
      Content-Type: application/sdp
      Content-Length: 164

      v=0
      o=- 2890844256 2890842807 IN IP4 172.16.2.93
      s=RTSP Session
      i=An Example of RTSP Session Usage
      a=control:rtsp://foo/twister
      t=0 0
      m=audio 0 RTP/AVP 0
      a=control:rtsp://foo/twister/audio
      m=video 0 RTP/AVP 26
      a=control:rtsp://foo/twister/video
```

### Setup Streams (Same Session)

```
C->M: SETUP rtsp://foo/twister/audio RTSP/1.0
      CSeq: 2
      Transport: RTP/AVP;unicast;client_port=8000-8001

M->C: RTSP/1.0 200 OK
      CSeq: 2
      Transport: RTP/AVP;unicast;client_port=8000-8001;
                 server_port=9000-9001
      Session: 12345678

C->M: SETUP rtsp://foo/twister/video RTSP/1.0
      CSeq: 3
      Transport: RTP/AVP;unicast;client_port=8002-8003
      Session: 12345678

M->C: RTSP/1.0 200 OK
      CSeq: 3
      Transport: RTP/AVP;unicast;client_port=8002-8003;
                 server_port=9004-9005
      Session: 12345678
```

### Play Aggregate

```
C->M: PLAY rtsp://foo/twister RTSP/1.0
      CSeq: 4
      Range: npt=0-
      Session: 12345678

M->C: RTSP/1.0 200 OK
      CSeq: 4
      Session: 12345678
      RTP-Info: url=rtsp://foo/twister/video;
                seq=9810092;rtptime=3450012
```

### Invalid: Pause Individual Stream

```
C->M: PAUSE rtsp://foo/twister/video RTSP/1.0
      CSeq: 5
      Session: 12345678

M->C: RTSP/1.0 460 Only Aggregate Operation Allowed
      CSeq: 5
```

### Valid: Pause Aggregate

```
C->M: PAUSE rtsp://foo/twister RTSP/1.0
      CSeq: 6
      Session: 12345678

M->C: RTSP/1.0 200 OK
      CSeq: 6
      Session: 12345678
```

### Invalid: SETUP on Aggregate URL

```
C->M: SETUP rtsp://foo/twister RTSP/1.0
      CSeq: 7
      Transport: RTP/AVP;unicast;client_port=10000

M->C: RTSP/1.0 459 Aggregate Operation Not Allowed
      CSeq: 7
```

---

## 14.3 Single Stream File

```
C->S: DESCRIBE rtsp://foo.com/test.wav RTSP/1.0
      CSeq: 1
      Accept: application/sdp

S->C: RTSP/1.0 200 OK
      CSeq: 1
      Content-base: rtsp://foo.com/test.wav/
      Content-type: application/sdp
      Content-length: 150

      v=0
      o=- 872653257 872653257 IN IP4 172.16.2.187
      s=mu-law wave file
      i=audio test
      t=0 0
      m=audio 0 RTP/AVP 0
      a=control:streamid=0

C->S: SETUP rtsp://foo.com/test.wav/streamid=0 RTSP/1.0
      CSeq: 2
      Transport: RTP/AVP/UDP;unicast;client_port=6970-6971;mode=play

S->C: RTSP/1.0 200 OK
      CSeq: 2
      Transport: RTP/AVP/UDP;unicast;client_port=6970-6971;
                 server_port=6970-6971;mode=play
      Session: 2034820394

C->S: PLAY rtsp://foo.com/test.wav RTSP/1.0
      CSeq: 3
      Session: 2034820394

S->C: RTSP/1.0 200 OK
      CSeq: 3
      Session: 2034820394
      RTP-Info: url=rtsp://foo.com/test.wav/streamid=0;
                seq=981888;rtptime=3781123
```

---

## 14.4 Live Multicast

Server chooses multicast address and port.

```
C->M: DESCRIBE rtsp://live.example.com/concert/audio RTSP/1.0
      CSeq: 1

M->C: RTSP/1.0 200 OK
      CSeq: 1
      Content-Type: application/sdp
      Content-Length: 150

      v=0
      o=- 2890844526 2890842807 IN IP4 192.16.24.202
      s=RTSP Session
      m=audio 3456 RTP/AVP 0
      a=control:rtsp://live.example.com/concert/audio
      c=IN IP4 224.2.0.1/16

C->M: SETUP rtsp://live.example.com/concert/audio RTSP/1.0
      CSeq: 2
      Transport: RTP/AVP;multicast

M->C: RTSP/1.0 200 OK
      CSeq: 2
      Transport: RTP/AVP;multicast;destination=224.2.0.1;
                 port=3456-3457;ttl=16
      Session: 0456804596

C->M: PLAY rtsp://live.example.com/concert/audio RTSP/1.0
      CSeq: 3
      Session: 0456804596

M->C: RTSP/1.0 200 OK
      CSeq: 3
      Session: 0456804596
```

Client joins multicast group 224.2.0.1 and listens on ports 3456/3457. See [SDP Usage](C-sdp-usage.md) for multicast address handling in SDP.

---

## 14.5 Playing into Existing Conference

Play media into existing conference (multicast destination known).

```
C->M: DESCRIBE rtsp://server.example.com/demo/548/sound RTSP/1.0
      CSeq: 1
      Accept: application/sdp

M->C: RTSP/1.0 200 OK
      CSeq: 1
      Content-Type: application/sdp

      v=0
      ...

C->M: SETUP rtsp://server.example.com/demo/548/sound RTSP/1.0
      CSeq: 2
      Transport: RTP/AVP;multicast;destination=224.2.36.42;
                 port=3456-3457;ttl=16
      Conference: 199702170042.SAA08642@obiwan.arl.wustl.edu%20Strstrdings

M->C: RTSP/1.0 200 OK
      CSeq: 2
      Transport: RTP/AVP;multicast;destination=224.2.36.42;
                 port=3456-3457;ttl=16
      Session: 91389234234
      Conference: 199702170042.SAA08642@obiwan.arl.wustl.edu%20Strstrdings
```

---

## 14.6 Recording

Record live audio stream.

```
C->M: ANNOUNCE rtsp://server.example.com/meeting RTSP/1.0
      CSeq: 1
      Content-Type: application/sdp

      v=0
      o=user 2890844526 2890842807 IN IP4 192.16.24.1
      s=Meeting Recording
      t=0 0
      m=audio 0 RTP/AVP 0
      a=control:streamid=0

M->C: RTSP/1.0 200 OK
      CSeq: 1

C->M: SETUP rtsp://server.example.com/meeting/streamid=0 RTSP/1.0
      CSeq: 2
      Transport: RTP/AVP;unicast;mode=record;client_port=4588-4589

M->C: RTSP/1.0 200 OK
      CSeq: 2
      Session: 93456789
      Transport: RTP/AVP;unicast;mode=record;
                 client_port=4588-4589;server_port=6256-6257

C->M: RECORD rtsp://server.example.com/meeting RTSP/1.0
      CSeq: 3
      Session: 93456789
      Range: npt=0-

M->C: RTSP/1.0 200 OK
      CSeq: 3
```

---

## Example Summary

| Example | Type | Servers | Control |
|---------|------|---------|---------|
| 14.1 | VoD Unicast | Multiple | Per-stream |
| 14.2 | Container | Single | Aggregate |
| 14.3 | Single Stream | Single | Mixed |
| 14.4 | Live Multicast | Single | N/A |
| 14.5 | Conference | Single | Conference |
| 14.6 | Recording | Single | N/A |

---

> [Back to Index](00-index.md) | [Previous: Caching](11-caching.md) | [Next: Syntax](13-syntax.md)
