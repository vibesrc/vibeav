# Section 8: SSRC Identifier Allocation and Use

> [Back to Index](00-index.md) | [Previous: Translators/Mixers](10-translators-mixers.md) | [Next: Security](12-security.md)

## SSRC Overview

The SSRC (Synchronization Source) is a 32-bit random identifier that MUST be globally unique within an RTP session.

### Why Random?

| Approach | Problem |
|----------|---------|
| IP address | Not unique (multiple sources per host, NAT) |
| Sequential | Conflicts in distributed systems |
| Time-based | Simultaneous starts cause collisions |
| **Random** | Low collision probability |

---

## 8.1 Probability of Collision

### Worst Case: Simultaneous Start

When N sources start simultaneously:

```
P(collision) ≈ 1 - exp(-N² / 2^(L+1))

Where:
  N = number of sources
  L = identifier length (32 bits)
```

| Sources | Collision Probability |
|---------|----------------------|
| 100 | ~10⁻⁶ |
| 1,000 | ~10⁻⁴ |
| 10,000 | ~10⁻² |

### Typical Case: Joining Existing Session

When one source joins N existing sources:

```
P(collision) = N / 2^L = N / 2^32
```

| Sources | Collision Probability |
|---------|----------------------|
| 1,000 | ~2×10⁻⁷ |
| 10,000 | ~2×10⁻⁶ |
| 100,000 | ~2×10⁻⁵ |

---

## 8.2 Collision Resolution

### Detection

Collision detected when packets arrive with:
- Same SSRC identifier
- Different source transport address

### Resolution Algorithm

```c
void handle_collision(uint32_t ssrc, transport_addr_t *addr, bool is_our_ssrc) {
    if (is_our_ssrc) {
        // Our SSRC collided - we must change
        send_bye(ssrc);
        our_ssrc = generate_random_ssrc();
        log_info("SSRC collision, changed to %08x", our_ssrc);
    } else {
        // Third-party collision - keep established source
        // Discard packets from new (conflicting) source
        add_to_conflict_list(ssrc, addr);
    }
}
```

### Self-Collision (Loop Detection)

If our own packets return (via translator/mixer loop):

1. Send BYE for old SSRC
2. Choose new SSRC
3. **Only do this once** per conflict address
4. If packets continue from same address, ignore them (it's a loop)

```c
typedef struct {
    transport_addr_t addr;
    bool bye_sent;
} conflict_entry_t;

void handle_own_collision(uint32_t ssrc, transport_addr_t *addr) {
    conflict_entry_t *entry = find_conflict(addr);

    if (entry == NULL) {
        // First collision from this address
        send_bye(ssrc);
        our_ssrc = generate_random_ssrc();
        add_conflict_entry(addr, true);
    } else if (entry->bye_sent) {
        // Already handled - it's a loop, ignore packets
        return;
    }
}
```

---

## 8.3 SSRC Generation

### Requirements

1. Use good random source
2. Check against known SSRCs before transmitting
3. Consider machine-specific entropy

### Algorithm

```c
uint32_t generate_random_ssrc(void) {
    uint32_t ssrc;

    do {
        // Combine multiple entropy sources
        ssrc = 0;

        // System random
        ssrc ^= (uint32_t)rand();

        // Process/thread ID
        ssrc ^= (uint32_t)getpid() << 16;

        // Current time (microseconds)
        struct timeval tv;
        gettimeofday(&tv, NULL);
        ssrc ^= (uint32_t)(tv.tv_usec);

        // Network address (partial)
        ssrc ^= get_local_addr() & 0xFFFF;

        // Mix with MD5 for better distribution
        ssrc = md5_mix(ssrc);

    } while (ssrc_in_use(ssrc) || ssrc == 0);

    return ssrc;
}
```

### MD5-Based Generation (Appendix A.6)

```c
uint32_t md5_generate_ssrc(void) {
    struct {
        struct timeval tv;
        clock_t cpu;
        pid_t pid;
        uid_t uid;
        uint32_t addr;
        char hostname[256];
    } entropy;

    gettimeofday(&entropy.tv, NULL);
    entropy.cpu = clock();
    entropy.pid = getpid();
    entropy.uid = getuid();
    entropy.addr = get_local_addr();
    gethostname(entropy.hostname, sizeof(entropy.hostname));

    uint8_t digest[16];
    MD5((uint8_t*)&entropy, sizeof(entropy), digest);

    return (digest[0] << 24) | (digest[1] << 16) |
           (digest[2] << 8)  | digest[3];
}
```

---

## Source Table Management

### Source Entry Structure

```c
typedef struct {
    uint32_t ssrc;
    char cname[256];
    transport_addr_t rtp_addr;
    transport_addr_t rtcp_addr;

    // Validation
    bool validated;
    int packet_count;

    // Reception statistics
    uint32_t max_seq;
    uint32_t cycles;
    uint32_t received;
    uint32_t expected_prior;
    uint32_t received_prior;

    // Timing
    double last_rtp_time;
    double last_rtcp_time;
    uint32_t last_rtp_ts;

    // SR info (for RTT calculation)
    uint32_t lsr;
    double lsr_arrival;
} rtp_source_t;
```

### Validation

New sources should be validated before counting:

```c
bool validate_source(rtp_source_t *src, rtp_packet_t *pkt) {
    if (src->validated) return true;

    src->packet_count++;

    // Require minimum packets or SDES CNAME
    if (src->packet_count >= MIN_SEQUENTIAL ||
        src->cname[0] != '\0') {
        src->validated = true;
        return true;
    }

    return false;
}
```

### Timeout

Sources timeout after 5× RTCP interval with no packets:

```c
void check_source_timeouts(rtp_session_t *session) {
    double timeout = 5 * session->rtcp_interval;
    double now = get_current_time();

    for (int i = 0; i < session->source_count; i++) {
        rtp_source_t *src = &session->sources[i];

        double last_activity = max(src->last_rtp_time, src->last_rtcp_time);

        if (now - last_activity > timeout) {
            if (src->validated) {
                session->members--;
            }
            remove_source(session, i);
            i--;  // Adjust index after removal
        }
    }

    // Reconsider RTCP interval if members changed
    reconsider_rtcp_interval(session);
}
```

---

## Collision Handling State Machine

```
                    ┌─────────────────────────────┐
                    │        NORMAL STATE         │
                    │    (using current SSRC)     │
                    └──────────────┬──────────────┘
                                   │
                    ┌──────────────┴──────────────┐
                    │ Receive packet with our     │
                    │ SSRC from different address │
                    └──────────────┬──────────────┘
                                   │
                    ┌──────────────┴──────────────┐
                    │   Check conflict table      │
                    └──────────────┬──────────────┘
                                   │
              ┌────────────────────┼────────────────────┐
              │                    │                    │
              ▼                    ▼                    ▼
    ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
    │  New conflict   │  │ Known conflict  │  │ Third-party     │
    │                 │  │ (loop detected) │  │ collision       │
    └────────┬────────┘  └────────┬────────┘  └────────┬────────┘
             │                    │                    │
             ▼                    ▼                    ▼
    ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
    │ Send BYE        │  │ Ignore packet   │  │ Keep established│
    │ Choose new SSRC │  │ (it's our loop) │  │ Discard new     │
    │ Add to conflict │  │                 │  │                 │
    └─────────────────┘  └─────────────────┘  └─────────────────┘
```

---

## CNAME and SSRC Binding

### Purpose

CNAME provides persistent identity across:
- SSRC changes (collision, restart)
- Multiple RTP sessions (audio + video)

### Detection via CNAME

```c
void check_cname_conflict(uint32_t ssrc, const char *cname,
                          transport_addr_t *addr) {
    rtp_source_t *existing = find_source_by_cname(cname);

    if (existing && existing->ssrc != ssrc) {
        // Same CNAME, different SSRC
        // This is normal for restarts - update the SSRC
        existing->ssrc = ssrc;
        existing->rtp_addr = *addr;
        log_info("Source %s changed SSRC to %08x", cname, ssrc);
    }
}
```

---

## Summary

| Aspect | Rule |
|--------|------|
| SSRC generation | 32-bit random, well-seeded |
| Collision detection | Same SSRC, different address |
| Own collision | Send BYE, choose new (once per address) |
| Third-party collision | Keep established, discard new |
| Loop detection | Repeated own-collision from same address |
| Validation | Multiple packets or SDES CNAME |
| Timeout | 5× RTCP interval |
| CNAME binding | Links SSRC across changes/sessions |

---

> [Back to Index](00-index.md) | [Previous: Translators/Mixers](10-translators-mixers.md) | [Next: Security](12-security.md)
