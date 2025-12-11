# RFC 3551: RTP Audio/Video Profile (AVP)

> [Back to Index](00-index.md) | [Previous: Security](12-security.md) | [Next: Algorithms](A1-algorithms.md)

## Overview

RFC 3551 defines the "RTP/AVP" profile for audio and video conferences with minimal control.

### Profile Features

| Feature | Value |
|---------|-------|
| Profile name | RTP/AVP |
| RTCP bandwidth | 5% of session |
| Sender share | 25% of RTCP (1.25% total) |
| Receiver share | 75% of RTCP (3.75% total) |
| Default RTP port | 5004 |
| Default RTCP port | 5005 |

---

## Payload Type Assignments

### Static Audio Payload Types

| PT | Encoding | Rate (Hz) | Channels | Description |
|----|----------|-----------|----------|-------------|
| 0 | PCMU | 8,000 | 1 | G.711 μ-law |
| 3 | GSM | 8,000 | 1 | GSM 06.10 |
| 4 | G723 | 8,000 | 1 | G.723.1 |
| 5 | DVI4 | 8,000 | 1 | IMA ADPCM |
| 6 | DVI4 | 16,000 | 1 | IMA ADPCM |
| 7 | LPC | 8,000 | 1 | LPC |
| 8 | PCMA | 8,000 | 1 | G.711 A-law |
| 9 | G722 | 8,000 | 1 | G.722 (actually 16kHz) |
| 10 | L16 | 44,100 | 2 | Linear PCM stereo |
| 11 | L16 | 44,100 | 1 | Linear PCM mono |
| 12 | QCELP | 8,000 | 1 | QCELP |
| 13 | CN | 8,000 | 1 | Comfort Noise |
| 14 | MPA | 90,000 | - | MPEG Audio |
| 15 | G728 | 8,000 | 1 | G.728 |
| 16 | DVI4 | 11,025 | 1 | IMA ADPCM |
| 17 | DVI4 | 22,050 | 1 | IMA ADPCM |
| 18 | G729 | 8,000 | 1 | G.729 |

### Static Video Payload Types

| PT | Encoding | Rate (Hz) | Description |
|----|----------|-----------|-------------|
| 25 | CelB | 90,000 | Sun CellB |
| 26 | JPEG | 90,000 | Motion JPEG |
| 28 | nv | 90,000 | Xerox PARC nv |
| 31 | H261 | 90,000 | H.261 |
| 32 | MPV | 90,000 | MPEG-1/2 Video |
| 33 | MP2T | 90,000 | MPEG-2 Transport |
| 34 | H263 | 90,000 | H.263 |

### Reserved and Dynamic

| Range | Usage |
|-------|-------|
| 1-2 | Reserved |
| 19 | Reserved (was CN) |
| 20-23 | Unassigned (audio) |
| 24, 27, 29-30 | Unassigned (video) |
| 35-71 | Unassigned |
| 72-76 | Reserved (RTCP conflict) |
| 77-95 | Unassigned |
| **96-127** | **Dynamic** |

---

## Dynamic Payload Types

### Usage

Dynamic payload types (96-127) are negotiated via:
- SDP (Session Description Protocol)
- H.323/H.245
- Other signaling protocols

### SDP Example

```
m=audio 5004 RTP/AVP 96 97 0
a=rtpmap:96 opus/48000/2
a=rtpmap:97 PCMU/8000
```

---

## Common Audio Encodings

### PCMU/PCMA (G.711)

| Property | Value |
|----------|-------|
| Sample rate | 8,000 Hz |
| Bits/sample | 8 |
| Bitrate | 64 kbps |
| Frame size | N/A (sample-based) |

```c
// PCMU (μ-law) encoding
uint8_t linear_to_ulaw(int16_t sample) {
    int sign = (sample >> 8) & 0x80;
    if (sign) sample = -sample;
    if (sample > 32635) sample = 32635;
    sample += 0x84;
    int exp = 7;
    for (int i = 0x4000; i > sample; i >>= 1, exp--);
    int mantissa = (sample >> (exp + 3)) & 0x0F;
    return ~(sign | (exp << 4) | mantissa);
}
```

### G.722

| Property | Value |
|----------|-------|
| Sample rate | 16,000 Hz (actual) |
| Clock rate | 8,000 Hz (RTP - historical error) |
| Bitrate | 64 kbps |

**Note**: The RTP clock rate of 8,000 Hz is incorrect but maintained for compatibility.

### Opus (Dynamic)

| Property | Value |
|----------|-------|
| Sample rates | 8k, 12k, 16k, 24k, 48k Hz |
| Channels | 1-2 (255 with extension) |
| Bitrate | 6-510 kbps |
| Clock rate | 48,000 Hz |

---

## Video Encodings

### Clock Rate

All video encodings use **90,000 Hz** (MPEG PTS frequency).

### Marker Bit Usage

```
Video: M=1 on last packet of frame
       M=0 on all other packets
```

### H.264 (Dynamic)

| Property | Value |
|----------|-------|
| Clock rate | 90,000 Hz |
| Packetization | RFC 6184 |
| Profiles | Baseline, Main, High, etc. |

### VP8/VP9 (Dynamic)

| Property | VP8 | VP9 |
|----------|-----|-----|
| Clock rate | 90,000 Hz | 90,000 Hz |
| RFC | 7741 | 7741 |

---

## Audio Encoding Rules

### Silence Suppression

Applications MAY suppress silence:
- Use marker bit (M=1) on first packet after silence
- Use CN (Comfort Noise) payload type during silence

### Packet Duration

```
Recommended: 20 ms
Minimum: Codec-dependent
Maximum: 200 ms (for interactive)
```

### Multiple Frames Per Packet

```
For frame-based codecs:
- Multiple frames MAY be packed in one packet
- All frames have same timestamp (first frame's sampling instant)
- Allows increased efficiency at cost of latency
```

---

## Minimum Implementation

### Required Support

| Type | Encoding |
|------|----------|
| Audio | PCMU (PT 0) |
| Audio | DVI4 (PT 5) |

### Interoperability

```
Without negotiation, use:
- PCMU at 8,000 Hz
- Single channel

With SDP negotiation:
- Use agreed codecs
- Dynamic payload types as mapped
```

---

## Port Assignment

### Defaults

| Purpose | Port |
|---------|------|
| RTP | 5004 (even) |
| RTCP | 5005 (odd) |

### Rules

1. RTP on even port number
2. RTCP on RTP port + 1
3. Applications MAY use any port pair
4. Port multiplexing allowed (RFC 5761)

---

## RTCP Configuration

### Bandwidth

```
Session bandwidth: B kbps
RTCP bandwidth: 0.05 × B = 5%

Sender RTCP: 0.0125 × B = 1.25%
Receiver RTCP: 0.0375 × B = 3.75%
```

### SDES Transmission

| Item | Frequency |
|------|-----------|
| CNAME | Every report |
| NAME | Every 3rd report (7/8 of extra slots) |
| EMAIL | Every 8th report (1/8 of extra slots) |
| Others | Remaining slots |

---

## Congestion Control

### Requirements

```
RTP flows MUST compete fairly with TCP:
- Monitor packet loss via RTCP RR
- Adapt bitrate accordingly
- Do not exceed TCP-fair rate
```

### Implementation

```c
void adapt_to_congestion(float loss_fraction, float rtt) {
    // TCP-friendly rate calculation (simplified)
    // Based on RFC 5348 equation
    float tcp_rate = (MTU / rtt) * sqrt(1.5 / loss_fraction);

    if (current_bitrate > tcp_rate) {
        // Reduce to TCP-fair rate
        set_bitrate(tcp_rate);
    }
}
```

---

## Summary

| Aspect | Value |
|--------|-------|
| Profile name | RTP/AVP |
| Default audio | PCMU (PT 0) |
| Default video | None (profile required) |
| Clock (audio) | Sampling rate |
| Clock (video) | 90,000 Hz |
| Dynamic PT range | 96-127 |
| RTCP bandwidth | 5% |
| Default ports | 5004/5005 |

---

> [Back to Index](00-index.md) | [Previous: Security](12-security.md) | [Next: Algorithms](A1-algorithms.md)
