# Appendix A: RTP Algorithms

> [Back to Index](00-index.md) | [Previous: AVP Profile](13-avp-profile.md) | [Next: Validation](A2-validation.md)

## A.1 RTP Data Header Validity Check

### Sequence Number Handling

```c
typedef struct {
    uint16_t max_seq;        // Highest seq number seen
    uint32_t cycles;         // Shifted count of seq number cycles
    uint32_t base_seq;       // Base seq number
    uint32_t bad_seq;        // Last bad seq + 1
    uint32_t probation;      // Packets until source valid
    uint32_t received;       // Packets received
    uint32_t expected_prior; // Packet expected at last interval
    uint32_t received_prior; // Packet received at last interval
} rtp_source_t;

#define RTP_SEQ_MOD (1 << 16)
#define MAX_DROPOUT 3000
#define MAX_MISORDER 100
#define MIN_SEQUENTIAL 2

void init_seq(rtp_source_t *s, uint16_t seq) {
    s->base_seq = seq;
    s->max_seq = seq;
    s->bad_seq = RTP_SEQ_MOD + 1;  // Invalid
    s->cycles = 0;
    s->received = 0;
    s->received_prior = 0;
    s->expected_prior = 0;
}

int update_seq(rtp_source_t *s, uint16_t seq) {
    uint16_t udelta = seq - s->max_seq;

    // Source not yet valid
    if (s->probation) {
        if (seq == s->max_seq + 1) {
            s->probation--;
            s->max_seq = seq;
            if (s->probation == 0) {
                init_seq(s, seq);
                s->received++;
                return 1;
            }
        } else {
            s->probation = MIN_SEQUENTIAL - 1;
            s->max_seq = seq;
        }
        return 0;
    } else if (udelta < MAX_DROPOUT) {
        // In order, with permissible gap
        if (seq < s->max_seq) {
            // Sequence number wrapped
            s->cycles += RTP_SEQ_MOD;
        }
        s->max_seq = seq;
    } else if (udelta <= RTP_SEQ_MOD - MAX_MISORDER) {
        // Large jump in sequence number
        if (seq == s->bad_seq) {
            // Two sequential packets, assume restart
            init_seq(s, seq);
        } else {
            s->bad_seq = (seq + 1) & (RTP_SEQ_MOD - 1);
            return 0;
        }
    } else {
        // Duplicate or misordered packet
    }
    s->received++;
    return 1;
}
```

---

## A.2 RTCP Header Validity Check

### Compound Packet Validation

```c
bool validate_rtcp_packet(uint8_t *buf, size_t len) {
    // Minimum RTCP packet: 8 bytes (empty RR)
    if (len < 8) return false;

    uint8_t *end = buf + len;
    uint8_t *p = buf;

    // First packet MUST be SR or RR
    uint8_t pt = p[1];
    if (pt != 200 && pt != 201) return false;

    // Check version on first packet
    uint8_t v = (p[0] >> 6) & 0x03;
    if (v != 2) return false;

    // Walk through compound packet
    while (p < end) {
        if (p + 4 > end) return false;

        v = (p[0] >> 6) & 0x03;
        if (v != 2) return false;

        // Length in 32-bit words minus 1
        uint16_t length = ((p[2] << 8) | p[3]) + 1;
        p += length * 4;
    }

    // Must end exactly at packet boundary
    return (p == end);
}
```

### Padding Validation

```c
bool validate_padding(uint8_t *buf, size_t len) {
    uint8_t padding = buf[0] & 0x20;
    if (!padding) return true;

    // Last byte contains padding count
    uint8_t pad_count = buf[len - 1];

    // Padding must be > 0 and <= remaining space
    if (pad_count == 0 || pad_count > len - 12) return false;

    // Padding must be multiple of 4
    if (pad_count % 4 != 0) return false;

    return true;
}
```

---

## A.3 Reception Statistics

### Computing Packet Loss

```c
void compute_reception_stats(rtp_source_t *s,
                             uint8_t *fraction_lost,
                             int32_t *cumulative_lost) {
    // Extended sequence number
    uint32_t extended_max = s->cycles + s->max_seq;
    uint32_t expected = extended_max - s->base_seq + 1;

    // Packets lost
    int32_t lost = expected - s->received;

    // Clamp to 24-bit signed
    if (lost > 0x7FFFFF) lost = 0x7FFFFF;
    if (lost < -0x800000) lost = -0x800000;
    *cumulative_lost = lost;

    // Fraction lost since last report
    uint32_t expected_interval = expected - s->expected_prior;
    s->expected_prior = expected;

    uint32_t received_interval = s->received - s->received_prior;
    s->received_prior = s->received;

    int32_t lost_interval = expected_interval - received_interval;
    if (expected_interval == 0 || lost_interval <= 0) {
        *fraction_lost = 0;
    } else {
        *fraction_lost = (lost_interval << 8) / expected_interval;
    }
}
```

---

## A.4 Jitter Calculation

### Interarrival Jitter

```c
typedef struct {
    uint32_t jitter;         // Current jitter estimate (fixed point)
    uint32_t last_rtp_ts;    // Last RTP timestamp received
    uint32_t last_arrival;   // Arrival time of last packet
    bool first_packet;
} jitter_state_t;

void update_jitter(jitter_state_t *j, uint32_t rtp_ts, uint32_t arrival) {
    if (j->first_packet) {
        j->first_packet = false;
        j->last_rtp_ts = rtp_ts;
        j->last_arrival = arrival;
        return;
    }

    // D = (arrival - last_arrival) - (rtp_ts - last_rtp_ts)
    // All in RTP timestamp units
    int32_t transit = arrival - rtp_ts;
    int32_t last_transit = j->last_arrival - j->last_rtp_ts;
    int32_t d = transit - last_transit;
    if (d < 0) d = -d;

    // J = J + (|D| - J) / 16
    j->jitter += d - ((j->jitter + 8) >> 4);

    j->last_rtp_ts = rtp_ts;
    j->last_arrival = arrival;
}

// Get jitter value for RTCP report
uint32_t get_jitter(jitter_state_t *j) {
    return j->jitter >> 4;  // Remove fixed-point fraction
}
```

---

## A.5 RTCP Transmission Interval

### Computing the Interval

```c
typedef struct {
    uint32_t members;        // Current member count
    uint32_t senders;        // Current sender count
    double rtcp_bw;          // RTCP bandwidth (bytes/s)
    double avg_rtcp_size;    // Average packet size
    bool we_sent;            // Have we sent recently?
    bool initial;            // First packet not yet sent?
} rtcp_state_t;

double compute_rtcp_interval(rtcp_state_t *s) {
    double t;
    double rtcp_min_time = 5.0;  // Seconds

    if (s->initial) {
        rtcp_min_time = 2.5;
    }

    // Number for calculation
    int n = s->members;

    // Sender/receiver split
    if (s->senders <= s->members * 0.25) {
        if (s->we_sent) {
            // Sender: use 25% of bandwidth
            double rtcp_sender_bw = s->rtcp_bw * 0.25;
            n = s->senders;
            t = s->avg_rtcp_size * n / rtcp_sender_bw;
        } else {
            // Receiver: use 75% of bandwidth
            double rtcp_receiver_bw = s->rtcp_bw * 0.75;
            n = s->members - s->senders;
            t = s->avg_rtcp_size * n / rtcp_receiver_bw;
        }
    } else {
        // > 25% senders: treat equally
        t = s->avg_rtcp_size * n / s->rtcp_bw;
    }

    // Apply minimum
    if (t < rtcp_min_time) {
        t = rtcp_min_time;
    }

    // Randomize [0.5, 1.5]
    t = t * (drand48() + 0.5);

    // Compensate for timer reconsideration
    t = t / 1.21828;

    return t;
}
```

### Timer Reconsideration

```c
void rtcp_timer_expired(rtcp_state_t *s) {
    double t = compute_rtcp_interval(s);
    double tc = get_current_time();

    if (s->tp + t <= tc) {
        // Time to send
        send_rtcp_packet();
        s->tp = tc;
        s->initial = false;

        // Update average size
        s->avg_rtcp_size = (1.0/16.0) * last_packet_size +
                           (15.0/16.0) * s->avg_rtcp_size;

        // Schedule next
        t = compute_rtcp_interval(s);
        s->tn = tc + t;
    } else {
        // Reconsider - delay sending
        s->tn = s->tp + t;
    }

    s->pmembers = s->members;
    schedule_timer(s->tn);
}
```

---

## A.6 SSRC Generation

### Random SSRC with MD5

```c
#include <openssl/md5.h>

uint32_t generate_ssrc(void) {
    struct {
        struct timeval tv;
        clock_t cpu;
        pid_t pid;
        uint32_t uid;
        struct utsname name;
        uint32_t addr;
    } entropy;

    gettimeofday(&entropy.tv, NULL);
    entropy.cpu = clock();
    entropy.pid = getpid();
    entropy.uid = getuid();
    uname(&entropy.name);
    entropy.addr = get_local_address();

    uint8_t digest[16];
    MD5((uint8_t*)&entropy, sizeof(entropy), digest);

    // Use first 4 bytes
    return (digest[0] << 24) | (digest[1] << 16) |
           (digest[2] << 8)  | digest[3];
}
```

---

## A.7 Reverse Reconsideration

### When Members Decrease

```c
void handle_bye_received(rtcp_state_t *s) {
    if (s->members < s->pmembers) {
        double tc = get_current_time();

        // Adjust tn (next transmission time)
        s->tn = tc + ((double)s->members / s->pmembers) * (s->tn - tc);

        // Adjust tp (last transmission time)
        s->tp = tc - ((double)s->members / s->pmembers) * (tc - s->tp);

        // Reschedule
        schedule_timer(s->tn);

        s->pmembers = s->members;
    }
}
```

---

## A.8 Complete Jitter Implementation

```c
// Full jitter calculation per RFC 3550 A.8
typedef struct {
    uint32_t jitter;
    uint32_t transit;
    int initialized;
} jitter_calc_t;

void jitter_init(jitter_calc_t *j) {
    j->jitter = 0;
    j->transit = 0;
    j->initialized = 0;
}

void jitter_update(jitter_calc_t *j, uint32_t rtp_ts, uint32_t arrival_ts) {
    // Convert arrival time to RTP timestamp units
    uint32_t transit = arrival_ts - rtp_ts;

    if (!j->initialized) {
        j->transit = transit;
        j->initialized = 1;
        return;
    }

    // D = |transit - last_transit|
    int32_t d = transit - j->transit;
    j->transit = transit;
    if (d < 0) d = -d;

    // J = J + (D - J) / 16
    // Using fixed-point: J stored as J*16
    j->jitter += d - ((j->jitter + 8) >> 4);
}

uint32_t jitter_get(jitter_calc_t *j) {
    // Return smoothed jitter value
    return j->jitter >> 4;
}
```

---

## Summary

| Algorithm | Purpose | Location |
|-----------|---------|----------|
| Sequence handling | Detect loss, reorder | A.1 |
| Packet validation | Reject invalid packets | A.2 |
| Loss calculation | Generate RR reports | A.3 |
| Jitter calculation | Network quality metric | A.4 |
| Interval computation | RTCP timing | A.5 |
| SSRC generation | Unique identifiers | A.6 |
| Reverse reconsideration | Adapt to BYE | A.7 |

---

> [Back to Index](00-index.md) | [Previous: AVP Profile](13-avp-profile.md) | [Next: Validation](A2-validation.md)
