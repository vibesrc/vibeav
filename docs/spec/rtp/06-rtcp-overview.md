# Sections 6.1-6.3: RTCP Overview

> [Back to Index](00-index.md) | [Previous: Multiplexing](05-multiplexing.md) | [Next: RTCP Reports](07-rtcp-reports.md)

## RTCP Purpose

RTCP (RTP Control Protocol) provides four functions:

| Function | Description | Required |
|----------|-------------|----------|
| **Quality feedback** | Reception reports (SR/RR) | SHOULD |
| **Participant ID** | CNAME for tracking | SHOULD |
| **Scaling** | Interval calculation | SHOULD |
| **Session control** | BYE, minimal info | OPTIONAL |

---

## RTCP vs RTP

| Aspect | RTP | RTCP |
|--------|-----|------|
| Port | Even (e.g., 5004) | Odd (e.g., 5005) |
| Content | Media data | Control info |
| Rate | Continuous | Periodic |
| Direction | Typically one-way | Bidirectional |

---

## 6.1 RTCP Packet Types

| Type | Value | Name | Purpose |
|------|-------|------|---------|
| SR | 200 | Sender Report | Stats from active senders |
| RR | 201 | Receiver Report | Stats from non-senders |
| SDES | 202 | Source Description | CNAME, NAME, etc. |
| BYE | 203 | Goodbye | Leave notification |
| APP | 204 | Application-defined | Custom data |

---

## Compound RTCP Packets

Multiple RTCP packets concatenated into one UDP packet.

### Structure

```
+----------------+----------------+----------------+
|  SR or RR      |     SDES       |   BYE/APP     |
+----------------+----------------+----------------+
          ↑               ↑               ↑
      Required        Required        Optional
```

### Rules

1. **First packet**: MUST be SR or RR (even if empty RR)
2. **SDES with CNAME**: MUST be included
3. **BYE**: SHOULD be last packet

### Example Compound Packet

```
if encrypted: 32-bit random prefix
|
|[-------- SR --------][---- SDES ----][-- BYE --]
|
|  sender info          CNAME item      reason
|  + reception rpts     + other items
|
+<------------------- UDP packet ------------------>
```

---

## 6.2 RTCP Transmission Interval

### Bandwidth Allocation

| Bandwidth | Purpose |
|-----------|---------|
| 5% of session | Total RTCP bandwidth |
| 25% of RTCP (1.25%) | For senders |
| 75% of RTCP (3.75%) | For receivers |

### Interval Calculation

Goal: Scale RTCP rate with participant count.

```
Minimum interval = 5 seconds (or shorter if allowed)

Actual interval = max(minimum, calculated_interval)

calculated_interval based on:
  - Number of participants
  - RTCP bandwidth
  - Average RTCP packet size
```

See [Algorithms](A3-algorithms.md) for computation details.

### Key Rules

- **Minimum interval**: 5 seconds (default)
- **First packet**: Random delay in [0.5×interval, 1.5×interval]
- **Scaling**: As participants increase, interval increases
- **Reconsideration**: Check before sending, may delay

---

## 6.3 RTCP Packet Send and Receive Rules

### Sending RTCP

1. Calculate transmission interval
2. Wait for interval
3. **Before sending**: Reconsider (recompute with current participant count)
4. Send compound packet
5. Update statistics

### Receiving RTCP

Track for each SSRC:
- Last SR timestamp (for RTT calculation)
- Packet/byte counts
- Participant activity

### Timing Out SSRCs

If no RTP or RTCP from SSRC for **5× RTCP interval**:
- Remove from participant list
- Update interval calculation

---

## Compound Packet Requirements

### Mandatory Components

```
+------------------------------------------+
| SR (if sender) or RR (if receiver-only)  |  ← Always first
+------------------------------------------+
| SDES with CNAME                          |  ← Always include
+------------------------------------------+
| Optional: Additional SDES, BYE, APP      |
+------------------------------------------+
```

### Size Constraints

- Keep compound packet ≤ MTU (typically 1500 bytes)
- If too many sources, round-robin across intervals
- Split into multiple compound packets if needed

---

## RTCP Packet Common Header

All RTCP packets share this format:

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|V=2|P|   RC    |      PT       |            length             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

| Field | Bits | Description |
|-------|------|-------------|
| V | 2 | Version (2) |
| P | 1 | Padding flag |
| RC | 5 | Report count (or subtype) |
| PT | 8 | Packet type (200-204) |
| Length | 16 | Packet length in 32-bit words minus 1 |

**Length note**: Length = 0 is valid (header only).

---

## Bandwidth Calculation Example

```
Session bandwidth: 1 Mbps (128 KB/s)
RTCP bandwidth: 5% = 6.4 KB/s

Participants: 100
Average RTCP packet: 100 bytes

Interval = (100 participants × 100 bytes) / 6400 bytes/s
         = 1.56 seconds

But minimum is 5 seconds, so interval = 5 seconds
```

---

## Session Member Tracking

### Variables

```c
typedef struct {
    uint32_t members;       // Current member count
    uint32_t senders;       // Current sender count
    uint32_t rtcp_bw;       // RTCP bandwidth (bytes/s)
    double avg_rtcp_size;   // Average compound packet size
    double tp;              // Last transmission time
    double tn;              // Next scheduled time
    bool we_sent;           // Have we sent RTP recently?
    bool initial;           // First RTCP not yet sent
} rtcp_state_t;
```

### Events

| Event | Action |
|-------|--------|
| RTP received (new SSRC) | Add to members |
| RTCP received (new SSRC) | Add to members |
| RTCP RR received (our SSRC) | Update RTT |
| RTCP BYE received | Remove from members |
| Timeout (no packets) | Remove from members |

---

## Summary

| Aspect | Value/Rule |
|--------|------------|
| RTCP bandwidth | 5% of session bandwidth |
| Sender share | 25% of RTCP bandwidth |
| Receiver share | 75% of RTCP bandwidth |
| Minimum interval | 5 seconds |
| First packet | Random in [2.5s, 7.5s] |
| Compound packet | SR/RR + SDES minimum |
| Timeout | 5 × RTCP interval |

---

> [Back to Index](00-index.md) | [Previous: Multiplexing](05-multiplexing.md) | [Next: RTCP Reports](07-rtcp-reports.md)
