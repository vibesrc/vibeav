# Section 6.4: Sender and Receiver Reports

> [Back to Index](00-index.md) | [Previous: RTCP Overview](06-rtcp-overview.md) | [Next: RTCP SDES](08-rtcp-sdes.md)

## Report Types

| Type | Code | When Used |
|------|------|-----------|
| SR (Sender Report) | 200 | Sent by active senders |
| RR (Receiver Report) | 201 | Sent by receivers (non-senders) |

**Key difference**: SR includes 20-byte sender info section.

---

## 6.4.1 SR: Sender Report

### Packet Format

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|V=2|P|    RC   |   PT=SR=200   |             length            | header
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                         SSRC of sender                        |
+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
|              NTP timestamp, most significant word             | sender
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+  info
|             NTP timestamp, least significant word             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                         RTP timestamp                         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                     sender's packet count                     |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                      sender's octet count                     |
+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
|                 SSRC_1 (SSRC of first source)                 | report
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+  block
| fraction lost |       cumulative number of packets lost       |   1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|           extended highest sequence number received           |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                      interarrival jitter                      |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                         last SR (LSR)                         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                   delay since last SR (DLSR)                  |
+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
|                 SSRC_2 (SSRC of second source)                | report
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+  block
:                               ...                             :   2
```

---

## Sender Info Section (20 bytes)

| Field | Bytes | Description |
|-------|-------|-------------|
| NTP timestamp | 8 | Wallclock time when sent |
| RTP timestamp | 4 | Corresponding RTP timestamp |
| Packet count | 4 | Total RTP packets sent |
| Octet count | 4 | Total payload bytes sent |

### NTP/RTP Timestamp Relationship

**Critical for synchronization**:

```
RTCP SR at time T:
  NTP timestamp = wallclock at T
  RTP timestamp = media clock at T

This mapping allows:
  1. Converting RTP timestamps to real time
  2. Synchronizing audio and video
  3. Lip-sync across streams
```

See [RTSP RTP Interaction](../rtsp/B-rtp-interaction.md) for RTSP usage.

---

## 6.4.2 RR: Receiver Report

Same as SR but without sender info section.

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|V=2|P|    RC   |   PT=RR=201   |             length            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                     SSRC of packet sender                     |
+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
|                         report block(s)                       |
|                              ...                              |
```

---

## Reception Report Block (24 bytes)

Each block reports on one source.

### Fields

| Field | Bits | Description |
|-------|------|-------------|
| SSRC_n | 32 | Source being reported |
| Fraction lost | 8 | Loss since last report (×256) |
| Cumulative lost | 24 | Total packets lost |
| Highest seq | 32 | Extended sequence number |
| Jitter | 32 | Interarrival jitter |
| LSR | 32 | Last SR timestamp (middle 32 bits) |
| DLSR | 32 | Delay since last SR (1/65536 sec) |

---

## Field Details

### Fraction Lost

```
fraction_lost = (packets_lost_this_interval / packets_expected) × 256
```

| Value | Meaning |
|-------|---------|
| 0 | No loss |
| 25 | ~10% loss |
| 51 | ~20% loss |
| 255 | ~100% loss |

### Cumulative Packets Lost

```
lost = expected - received

expected = highest_seq - initial_seq
received = actual packets received
```

**Can be negative** (if duplicates received).

### Extended Highest Sequence Number

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|           cycles              |        highest seq            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

- **Cycles**: Number of times seq wrapped (0, 1, 2, ...)
- **Highest seq**: Last seq number received

### Interarrival Jitter

Mean deviation of packet spacing.

```
For packets i and j:
  D(i,j) = (Rj - Ri) - (Sj - Si)

Where:
  Ri, Rj = receive times (in timestamp units)
  Si, Sj = RTP timestamps

Jitter updated per packet:
  J = J + (|D| - J) / 16
```

See [Algorithms](A3-algorithms.md#jitter-calculation) for implementation.

### LSR (Last SR)

Middle 32 bits of NTP timestamp from last SR received.

```
Full NTP: 0x83AA7E80_80000000
LSR:      0x7E808000
```

If no SR received: LSR = 0

### DLSR (Delay Since Last SR)

Time between receiving SR and sending this RR.

```
Units: 1/65536 seconds (~15μs resolution)

Example: 1.5 seconds delay
  DLSR = 1.5 × 65536 = 98304 = 0x00018000
```

---

## Round-Trip Time Calculation

Sender can calculate RTT from reception reports:

```
RTT = A - LSR - DLSR

Where:
  A = current NTP time (compact, middle 32 bits)
  LSR = from reception report
  DLSR = from reception report
```

### Example

```
Sender:
  Sent SR with NTP = 0xB705:2000 at T1

Receiver:
  Received SR at T2
  Sent RR at T3 = T2 + 5.25 seconds
  LSR = 0xB705:2000
  DLSR = 5.25 × 65536 = 0x0005:4000

Sender receives RR at T4:
  A = 0xB710:8000 (current time, compact)
  RTT = 0xB710:8000 - 0xB705:2000 - 0x0005:4000
      = 0x0006:2000
      = 6.125 seconds
```

---

## Report Block Limits

- **RC field**: 5 bits = max 31 report blocks
- If >31 sources: Stack additional RR packets
- Round-robin if exceeding MTU

---

## Implementation

### Generating SR

```c
void generate_sr(rtcp_sr_t *sr, rtp_session_t *session) {
    sr->version = 2;
    sr->padding = 0;
    sr->rc = count_sources_heard(session);
    sr->pt = 200;  // SR
    sr->ssrc = session->our_ssrc;

    // Sender info
    get_ntp_timestamp(&sr->ntp_sec, &sr->ntp_frac);
    sr->rtp_ts = get_current_rtp_timestamp(session);
    sr->packet_count = session->packets_sent;
    sr->octet_count = session->octets_sent;

    // Add reception report blocks
    for (int i = 0; i < sr->rc; i++) {
        fill_report_block(&sr->blocks[i], &session->sources[i]);
    }

    sr->length = 6 + (sr->rc * 6);  // In 32-bit words minus 1
}
```

### Processing SR (for sync)

```c
void process_sr(rtcp_sr_t *sr, rtp_source_t *source) {
    source->ntp_sec = sr->ntp_sec;
    source->ntp_frac = sr->ntp_frac;
    source->rtp_ts_at_sr = sr->rtp_ts;
    source->lsr = ntp_to_compact(sr->ntp_sec, sr->ntp_frac);
    source->lsr_arrival = get_ntp_compact_now();
}
```

---

## Summary

| Report | Size | Contents |
|--------|------|----------|
| SR header | 8 bytes | V, P, RC, PT, length, SSRC |
| Sender info | 20 bytes | NTP, RTP ts, counts |
| RR header | 8 bytes | V, P, RC, PT, length, SSRC |
| Report block | 24 bytes | Per-source stats |

**Key uses**:
- NTP/RTP mapping → synchronization
- Fraction lost → quality indicator
- Jitter → network stability
- LSR/DLSR → RTT calculation

---

> [Back to Index](00-index.md) | [Previous: RTCP Overview](06-rtcp-overview.md) | [Next: RTCP SDES](08-rtcp-sdes.md)
