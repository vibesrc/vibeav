# Sections 9-10: Security and Congestion

> [Back to Index](00-index.md) | [Previous: SSRC](11-ssrc.md) | [Next: AVP Profile](13-avp-profile.md)

## 9. Security Considerations

### RTP Security Services

| Service | Default | Description |
|---------|---------|-------------|
| Confidentiality | Optional | Encrypt payload |
| Authentication | Optional | Verify source |
| Integrity | Optional | Detect tampering |

### Default: No Security

By default, RTP/RTCP packets are sent unencrypted.

---

## 9.1 Confidentiality

### Encryption Scope

Two options:
1. **Entire packet** (after fixed header)
2. **Payload only** (header remains clear)

### Encryption Method

```
If encrypting entire packet:
  - Add 32-bit random prefix for initialization
  - Encrypt everything after RTP header

Compound RTCP:
  - Prepend 32-bit random value
  - Encrypt as single unit
```

### Packet Structure (Encrypted)

```
Encrypted RTP:
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                    32-bit random prefix                       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|V=2|P|X|  CC   |M|     PT      |       sequence number         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                           timestamp                           |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                             SSRC                              |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                      encrypted portion                       ...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
          ↑ Random prefix ensures unique ciphertext
```

### Key Management

Key distribution is **out of scope** for RTP. Options include:
- Pre-shared keys
- Session Description Protocol (SDP) extensions
- DTLS-SRTP (modern approach)

---

## 9.2 Authentication and Message Integrity

### SRTP (Secure RTP)

RFC 3711 defines SRTP for modern security:

| Feature | SRTP |
|---------|------|
| Encryption | AES-CTR or AES-GCM |
| Authentication | HMAC-SHA1 |
| Key derivation | Cryptographic |
| Replay protection | Yes |

### Authentication Tag

```
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                          RTP packet                          ...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                     Authentication tag                       ...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

---

## Security Threats

### Threat Model

| Threat | Risk | Mitigation |
|--------|------|------------|
| Eavesdropping | High | Encryption |
| Replay attack | Medium | Sequence validation |
| Injection | Medium | Authentication |
| DoS (flood) | High | Rate limiting |
| SSRC collision attack | Low | Collision resolution |

### SSRC-Based Attacks

An attacker could:
1. Send packets with victim's SSRC → force victim to change SSRC
2. Flood BYE packets → remove participants
3. Inject false SDES → confuse identification

**Mitigation**: Use SRTP with authentication.

---

## 10. Congestion Control

### RTP and Congestion

RTP itself does not provide congestion control. Applications MUST:
1. Monitor packet loss (via RTCP RR)
2. Adapt transmission rate accordingly
3. Not consume more bandwidth than TCP would

### AVP Profile Guidance

From RFC 3551:

```
Applications SHOULD monitor packet loss to ensure that the
packet loss rate is within acceptable parameters. Packet loss
is considered acceptable if a TCP flow across the same network
path would achieve comparable throughput.
```

### Adaptation Strategies

| Strategy | Description |
|----------|-------------|
| Rate adaptation | Reduce bitrate when loss increases |
| Codec switching | Change to lower-rate codec |
| FEC | Add redundancy instead of reducing rate |
| Layered coding | Drop enhancement layers |

### Monitoring Loss

```c
void check_congestion(rtp_source_t *src, rtcp_rr_t *rr) {
    // Fraction lost is 8-bit fixed point (×256)
    float loss_fraction = rr->fraction_lost / 256.0f;

    if (loss_fraction > 0.10) {
        // >10% loss - significant congestion
        reduce_bitrate(REDUCTION_LARGE);
    } else if (loss_fraction > 0.02) {
        // 2-10% loss - moderate congestion
        reduce_bitrate(REDUCTION_SMALL);
    } else if (loss_fraction < 0.01) {
        // <1% loss - may increase
        increase_bitrate(INCREMENT_SMALL);
    }
}
```

---

## RTCP Bandwidth Security

### Limiting RTCP Bandwidth

RTCP is limited to 5% of session bandwidth to prevent control traffic from overwhelming data.

### BYE Flood Prevention

Large sessions (>50 members) use scaled BYE transmission:

```c
void send_bye_with_backoff(rtp_session_t *s) {
    if (s->members > 50) {
        // Use BYE backoff algorithm
        // Reset members count, track only BYE packets
        s->tp = s->tc;
        s->members = 1;
        s->pmembers = 1;
        s->senders = 0;
        s->initial = true;
        s->avg_rtcp_size = bye_packet_size;

        // Schedule BYE using normal interval calculation
        s->tn = s->tc + compute_rtcp_interval(s);
    } else {
        // Small session - send immediately
        send_bye_packet();
    }
}
```

---

## Implementation Notes

### Minimum Security Recommendations

For production use:
1. **Use SRTP** (RFC 3711) instead of basic RTP
2. **Use DTLS-SRTP** for key exchange
3. **Validate SSRC/CNAME** consistency
4. **Implement rate limiting** on received packets

### Basic Validation

```c
bool validate_rtp_packet(uint8_t *buf, size_t len, rtp_source_t *src) {
    if (len < 12) return false;  // Too short

    uint8_t version = (buf[0] >> 6) & 0x03;
    if (version != 2) return false;  // Wrong version

    // Check source transport address matches known source
    if (src && !addr_matches(&src->rtp_addr, &packet_addr)) {
        // Possible collision or attack
        handle_possible_collision(src);
        return false;
    }

    return true;
}
```

---

## Summary

| Aspect | Recommendation |
|--------|----------------|
| Encryption | Use SRTP (RFC 3711) |
| Authentication | Use SRTP with authentication |
| Key exchange | Use DTLS-SRTP |
| Congestion control | Application responsibility |
| Loss monitoring | Use RTCP RR fraction_lost |
| BYE flooding | Use backoff for large sessions |
| SSRC attacks | Validate with authentication |

---

> [Back to Index](00-index.md) | [Previous: SSRC](11-ssrc.md) | [Next: AVP Profile](13-avp-profile.md)
