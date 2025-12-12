# Section 4: Byte Order, Alignment, and Time Format

> [Back to Index](00-index.md) | [Previous: Definitions](02-definitions.md) | [Next: RTP Header](04-rtp-header.md)

## Byte Order

All integer fields are **big-endian** (network byte order).

```
Most Significant Byte first

Example: 32-bit value 0x12345678

Byte 0: 0x12  (MSB)
Byte 1: 0x34
Byte 2: 0x56
Byte 3: 0x78  (LSB)
```

### Implementation

```c
// Writing 32-bit value
void write_uint32(uint8_t *buf, uint32_t val) {
    buf[0] = (val >> 24) & 0xFF;
    buf[1] = (val >> 16) & 0xFF;
    buf[2] = (val >> 8) & 0xFF;
    buf[3] = val & 0xFF;
}

// Reading 32-bit value
uint32_t read_uint32(uint8_t *buf) {
    return (buf[0] << 24) | (buf[1] << 16) | (buf[2] << 8) | buf[3];
}
```

---

## Alignment

All header data is aligned to its natural length:

| Field Size | Alignment |
|------------|-----------|
| 16-bit | Even byte offset |
| 32-bit | Offset divisible by 4 |

**Padding octets** have value zero.

---

## NTP Timestamp Format

Wallclock time is represented in NTP format.

### Full NTP Timestamp (64 bits)

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                   Seconds (integer part)                      |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                   Seconds (fractional part)                   |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

| Part | Bits | Description |
|------|------|-------------|
| Integer | 32 | Seconds since Jan 1, 1900 |
| Fraction | 32 | Fractional seconds |

**Resolution**: ~200 picoseconds

### Compact NTP Timestamp (32 bits)

Used in RTCP receiver reports (LSR, DLSR fields).

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|     Low 16 bits of integer    |   High 16 bits of fraction    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

Takes middle 32 bits of full 64-bit NTP timestamp.

**Resolution**: ~15 microseconds

---

## NTP Epoch

**Epoch**: January 1, 1900, 00:00:00 UTC

### Conversion Examples

```
Unix timestamp (seconds since 1970) to NTP:
  ntp_sec = unix_sec + 2208988800  // 70 years in seconds

NTP to Unix:
  unix_sec = ntp_sec - 2208988800
```

### Wraparound

NTP timestamps will wrap in year **2036**.

**For RTP purposes**: Only differences matter, so wraparound is handled with modular arithmetic (works if timestamps within 68 years).

---

## Time Sources

### Using NTP

If running NTP:
- Use system NTP time
- Enables inter-host synchronization
- Required for multi-stream lip-sync across machines

### Without NTP

May use any consistent clock:
- System uptime
- Monotonic clock
- Must be consistent across application's streams

### No Clock

If no wallclock available:
- May set NTP timestamp to zero in RTCP SR
- Disables inter-media synchronization

---

## RTP vs NTP Timestamps

| Aspect | RTP Timestamp | NTP Timestamp |
|--------|---------------|---------------|
| Where | RTP data packets | RTCP SR packets |
| Clock | Media clock | Wallclock |
| Rate | Codec-dependent | Fixed |
| Purpose | Jitter, sequencing | Synchronization |

### Relationship

RTCP SR contains both:

```
RTCP Sender Report:
  NTP timestamp: 3851234567.123456789  (wallclock)
  RTP timestamp: 12345678               (media clock)

This mapping enables:
  - Converting RTP timestamps to real time
  - Synchronizing audio and video
```

### Clock Rate Examples

| Media | Clock Rate | Meaning |
|-------|------------|---------|
| G.711 audio | 8000 Hz | 1 tick = 125 μs |
| Opus audio | 48000 Hz | 1 tick = 20.8 μs |
| H.264 video | 90000 Hz | 1 tick = 11.1 μs |

---

## Implementation Notes

### Generating NTP Timestamp

```c
#include <time.h>

void get_ntp_timestamp(uint32_t *ntp_sec, uint32_t *ntp_frac) {
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);

    // Convert Unix to NTP epoch
    *ntp_sec = ts.tv_sec + 2208988800UL;

    // Convert nanoseconds to NTP fraction
    // frac = nsec * 2^32 / 10^9
    *ntp_frac = (uint32_t)(((uint64_t)ts.tv_nsec << 32) / 1000000000ULL);
}
```

### Compact NTP (Middle 32 bits)

```c
uint32_t ntp_to_compact(uint32_t ntp_sec, uint32_t ntp_frac) {
    return ((ntp_sec & 0xFFFF) << 16) | ((ntp_frac >> 16) & 0xFFFF);
}
```

### RTP Timestamp from Media Clock

```c
// For 8kHz audio, 20ms packets
uint32_t rtp_timestamp = 0;
const uint32_t samples_per_packet = 160;  // 8000 * 0.020

void send_audio_packet(uint8_t *samples) {
    create_rtp_packet(rtp_timestamp, samples);
    rtp_timestamp += samples_per_packet;
}
```

---

## Summary

| Aspect | Format |
|--------|--------|
| Byte order | Big-endian (network order) |
| Alignment | Natural (16-bit on even, 32-bit on 4) |
| NTP full | 64-bit: 32 int + 32 frac |
| NTP compact | 32-bit: middle of full |
| NTP epoch | Jan 1, 1900 |
| Padding | Zero bytes |

---

> [Back to Index](00-index.md) | [Previous: Definitions](02-definitions.md) | [Next: RTP Header](04-rtp-header.md)
