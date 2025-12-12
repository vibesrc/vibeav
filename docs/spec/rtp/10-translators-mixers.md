# Section 7: RTP Translators and Mixers

> [Back to Index](00-index.md) | [Previous: RTCP BYE/APP](09-rtcp-bye-app.md) | [Next: SSRC](11-ssrc.md)

## Overview

Translators and mixers are intermediate systems that connect RTP sessions.

| Type | Function | SSRC Handling |
|------|----------|---------------|
| **Translator** | Forwards packets, may change encoding | Keeps original SSRC |
| **Mixer** | Combines multiple streams into one | Uses its own SSRC, adds CSRC list |

---

## 7.1 General Description

### Transport-Level Clouds

A translator/mixer connects two or more "clouds":
- Each cloud = network protocol + multicast address + port pair
- OR unicast address pairs

### Loop Prevention Rules

1. Each cloud MUST be distinct (protocol, address, or port)
2. NO parallel translators/mixers unless sources are partitioned
3. All connected end systems share the same SSRC space

---

## Translator Behavior

### Data Handling

```
Source: E1 (SSRC=0x12345678)
        ↓
   [Translator T1]
        ↓
Forwarded: SSRC=0x12345678 (unchanged)
```

### Modifications Allowed

| Modification | SSRC | Timestamp | Sequence |
|--------------|------|-----------|----------|
| Pass-through | Keep | Keep | Keep |
| Re-encode | Keep | Modify if clock changes | Modify if packets combined |
| Multi→Uni | Keep | Keep | Keep |

### Key Rules

- SSRC identifier: **unchanged**
- Payload type: MAY change (if encoding changes)
- Timestamp: MUST change if timestamp frequency changes
- Sequence: MUST reassign if packets are combined/split

---

## Mixer Behavior

### Data Handling

```
Source E1: SSRC=0x11111111 ─┐
                            │
Source E2: SSRC=0x22222222 ─┼──► [Mixer M1] ──► SSRC=0xABCD0000
                            │                   CSRC=[0x11111111, 0x22222222]
Source E3: SSRC=0x33333333 ─┘
```

### Key Rules

- **New SSRC**: Mixer generates its own SSRC
- **CSRC List**: Contains SSRCs of contributing sources
- **Timing**: Mixer generates new timing (is the sync source)
- **Max CSRCs**: 15 (CC field is 4 bits)

### Advantages/Disadvantages

| Aspect | Advantage | Disadvantage |
|--------|-----------|--------------|
| Bandwidth | Limited to one stream | - |
| Control | - | Receivers can't select sources |
| Sync | - | Loses original sync info |

---

## 7.2 RTCP Processing in Translators

### SR Sender Information

| Field | Action |
|-------|--------|
| SSRC | Keep unchanged |
| NTP timestamp | Keep unchanged |
| RTP timestamp | Change if clock rate changes |
| Packet count | Change if packets combined |
| Octet count | Change if encoding changes |

### SR/RR Reception Reports

- Forward in **opposite direction** of data
- SSRC: Keep unchanged
- Sequence numbers: Inverse mapping if modified
- May synthesize reports if translation makes original meaningless

### SDES

- Forward CNAME (required for collision detection)
- May filter other items for bandwidth

### BYE

- Forward unchanged
- On shutdown: Send BYE for all forwarded SSRCs

### APP

- Forward unchanged

---

## 7.3 RTCP Processing in Mixers

### SR Sender Information

- **Generate own SR** for mixed stream
- Do NOT forward source SRs (characteristics lost in mix)

### SR/RR Reception Reports

```
Cloud A                    Mixer                    Cloud B
   ←── RR (for Cloud A sources) ←──
                              ──► RR (for Cloud B sources) ──►
```

- Generate **separate reports** for each cloud
- Report only on sources in that cloud
- Do NOT forward reports between clouds

### SDES

- Forward CNAMEs (for collision detection)
- May aggregate chunks into single SDES packet
- Mixer MUST include own CNAME

### BYE

- Forward BYE packets
- On shutdown: Send BYE for all CSRCs + own SSRC

### APP

- Application-specific handling

---

## 7.4 Cascaded Mixers

When mixers are chained:

```
[E1] → [M1] → [M2] → Output
       ↓      ↓
     CSRC:  CSRC:
     [E1]  [E1, M1's other sources]
```

### CSRC List Building

```c
// Building CSRC list for cascaded mixer
void build_csrc_list(rtp_packet_t *out, rtp_packet_t **inputs, int count) {
    int csrc_count = 0;

    for (int i = 0; i < count && csrc_count < 15; i++) {
        rtp_packet_t *in = inputs[i];

        if (in->cc > 0) {
            // Already mixed - copy CSRCs
            for (int j = 0; j < in->cc && csrc_count < 15; j++) {
                out->csrc[csrc_count++] = in->csrc[j];
            }
        } else {
            // Direct source - use SSRC
            out->csrc[csrc_count++] = in->ssrc;
        }
    }

    out->cc = csrc_count;
}
```

---

## Network Diagram Example

```
[E1]                                    [E6]
 |                                       |
 E1:17                             E6:15 |
 |                                       |   E6:15
 V  M1:48 (1,17)         M1:48 (1,17)   V   M1:48 (1,17)
(M1)-------------><T1>-----------------><T2>-------------->[E7]
 ^                 ^     E4:47          ^   E4:47
 | E2:1      E4:47 |                    |   M3:89 (64,45)
 |                 |                    |
[E2]              [E4]    M3:89 (64,45) |
                                        |
[E3] --------->(M2)----------->(M3)-----|
       E3:64       M2:12 (64)   ^
                                | E5:45
                               [E5]

Legend:
  [E] = End system
  (M) = Mixer
  <T> = Translator
  M1:48 (1,17) = Mixer M1, SSRC=48, CSRC=[1,17]
```

---

## Implementation Considerations

### Translator Implementation

```c
typedef struct {
    uint32_t ssrc;           // Not our own - just tracking
    int encoding_in;         // Source encoding
    int encoding_out;        // Target encoding
    uint32_t clock_in;       // Source clock rate
    uint32_t clock_out;      // Target clock rate
} translator_t;

void translate_rtp(translator_t *t, rtp_packet_t *in, rtp_packet_t *out) {
    // Copy header, keep SSRC
    out->ssrc = in->ssrc;

    // Adjust timestamp if clock rate changed
    if (t->clock_in != t->clock_out) {
        out->timestamp = (uint32_t)(
            (uint64_t)in->timestamp * t->clock_out / t->clock_in
        );
    } else {
        out->timestamp = in->timestamp;
    }

    // Re-encode payload if needed
    if (t->encoding_in != t->encoding_out) {
        transcode_payload(in, out, t->encoding_in, t->encoding_out);
        out->pt = get_payload_type(t->encoding_out);
    } else {
        memcpy(out->payload, in->payload, in->payload_len);
        out->pt = in->pt;
    }
}
```

### Mixer Implementation

```c
typedef struct {
    uint32_t our_ssrc;
    uint32_t csrc_list[15];
    int csrc_count;
    uint32_t timestamp;
    uint16_t sequence;
} mixer_t;

void mix_audio(mixer_t *m, audio_frame_t **frames, int count,
               rtp_packet_t *out) {
    // Mix audio samples
    int16_t mixed[FRAME_SIZE] = {0};
    m->csrc_count = 0;

    for (int i = 0; i < count && m->csrc_count < 15; i++) {
        for (int j = 0; j < FRAME_SIZE; j++) {
            mixed[j] += frames[i]->samples[j];
        }
        m->csrc_list[m->csrc_count++] = frames[i]->ssrc;
    }

    // Clip to prevent overflow
    for (int j = 0; j < FRAME_SIZE; j++) {
        if (mixed[j] > 32767) mixed[j] = 32767;
        if (mixed[j] < -32768) mixed[j] = -32768;
    }

    // Build output packet
    out->ssrc = m->our_ssrc;
    out->timestamp = m->timestamp;
    out->sequence = m->sequence++;
    out->cc = m->csrc_count;
    memcpy(out->csrc, m->csrc_list, m->csrc_count * sizeof(uint32_t));
    encode_audio(mixed, out->payload, &out->payload_len);

    m->timestamp += FRAME_SIZE;
}
```

---

## Summary

| Component | Translator | Mixer |
|-----------|------------|-------|
| SSRC | Preserves original | Generates new |
| CSRC | None | Lists contributors |
| SR/RR | Forwards (modified) | Generates own |
| SDES | Forwards | Forwards + own |
| BYE | Forwards | Forwards |
| Timing | May modify | Generates new |

---

> [Back to Index](00-index.md) | [Previous: RTCP BYE/APP](09-rtcp-bye-app.md) | [Next: SSRC](11-ssrc.md)
