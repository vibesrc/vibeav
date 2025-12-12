# Section 16: Security Considerations

> [Back to Index](00-index.md) | [Previous: Syntax](13-syntax.md) | [Next: State Machines](A-state-machines.md)

## Overview

RTSP inherits many security considerations from HTTP, with additional concerns for real-time media streaming.

---

## Authentication

RTSP uses HTTP authentication mechanisms (RFC 2069). See [Headers - Authentication](09-headers.md#authentication-headers) for header details.

#### Basic Authentication

```
Authorization: Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==
```

**Warning**: Basic auth sends credentials in base64 (NOT encrypted). Use only over TLS.

#### Digest Authentication

```
WWW-Authenticate: Digest realm="example",
                  nonce="dcd98b7102dd2f0e8b11d0f600bfb0c093",
                  opaque="5ccc069c403ebaf9f0171e9517f40e41"

Authorization: Digest username="user",
               realm="example",
               nonce="dcd98b7102dd2f0e8b11d0f600bfb0c093",
               uri="rtsp://server/movie",
               response="e966c932a9242554e42c8ee200cec7f6",
               opaque="5ccc069c403ebaf9f0171e9517f40e41"
```

Digest is preferred over Basic.

### Proxy Authentication

```
Proxy-Authenticate: Basic realm="Proxy"
Proxy-Authorization: Basic dXNlcjpwYXNz
```

---

## Denial of Service Attacks

### Amplification Attack

**Threat**: Client requests media sent to victim's address.

```
SETUP rtsp://server/movie RTSP/1.0
Transport: RTP/AVP;unicast;destination=VICTIM_IP;client_port=9999
```

**Mitigations**:
1. **Verify source**: Only send to requesting IP
2. **Authenticate**: Require authentication before SETUP
3. **Log requests**: Track suspicious destination requests
4. **Rate limit**: Limit streams per client

### Resource Exhaustion

**Threat**: Client opens many sessions without cleanup.

**Mitigations**:
1. **Session timeout**: Enforce session expiration
2. **Limits**: Maximum sessions per client
3. **RTCP monitoring**: Detect inactive clients

---

## Transport Security

### Eavesdropping

**Threat**: Media/control data intercepted.

**Mitigations**:
1. **RTSPS**: RTSP over TLS (rtpss://)
2. **SRTP**: Secure RTP for media
3. **IPsec**: Network-level encryption

### Session Hijacking

**Threat**: Attacker steals session ID.

**Mitigations**:
1. **Random session IDs**: Cryptographically random, 8+ characters
2. **TLS**: Encrypt RTSP traffic
3. **IP binding**: Validate source IP
4. **Short timeouts**: Limit session lifetime

---

## Session ID Security

### Requirements

- **Length**: At least 8 characters
- **Randomness**: Cryptographically random
- **Unpredictable**: Not based on timestamp, PID, etc.

### Good Session ID

```
Session: A3Xk9B2mQ7nPf1Yz
```

### Bad Session ID

```
Session: 12345678        # Sequential
Session: user_1234       # Predictable
Session: 1638547200      # Timestamp
```

---

## Media Security

### RTP/RTCP Security

Standard RTP provides no security. Options:

| Protocol | Description |
|----------|-------------|
| SRTP | Secure RTP (RFC 3711) |
| SRTCP | Secure RTCP |
| IPsec | Network-layer security |

### Key Exchange

Methods for distributing media encryption keys:

1. **Out-of-band**: Via RTSP (encrypted)
2. **SDP crypto attribute**: Key in SDP
3. **MIKEY**: Multimedia Internet KEYing

---

## URL Security

### Absolute URLs

RTSP requires absolute URLs, preventing some HTTP attacks.

### Sensitive Information in URLs

**Avoid**:
```
rtsp://server/movie?auth=secret123
rtsp://user:pass@server/movie
```

Use Authorization header instead.

---

## Content Security

### Presentation Description

- **Validate**: Check SDP for malicious content
- **Limit**: Restrict number of streams
- **Verify**: Confirm URLs match expected servers

### Redirect Security

```
RTSP/1.0 302 Moved Temporarily
Location: rtsp://evil.server/attack
```

**Mitigations**:
1. Limit redirect hops
2. Verify domain whitelist
3. Prompt user for external redirects

---

## Server Security

### Input Validation

| Input | Validation |
|-------|------------|
| URL | Check format, length, characters |
| Headers | Limit count, size |
| Body | Verify Content-Length, Content-Type |
| Ranges | Check bounds |
| Transport | Validate parameters |

### Resource Protection

- **Rate limiting**: Requests per second
- **Connection limits**: Max concurrent connections
- **Bandwidth limits**: Max streams per client
- **Access control**: Path-based permissions

---

## Client Security

### Server Verification

- **Certificates**: Verify TLS certificates
- **Domain checking**: Match expected server
- **Redirect limits**: Prevent infinite loops

### Local Resource Protection

- **File access**: Sandbox media files
- **Memory**: Limit buffer sizes
- **CPU**: Timeout for processing

---

## Security Checklist

### Server Implementation

- [ ] Use Digest over Basic authentication
- [ ] Implement TLS (RTSPS)
- [ ] Generate cryptographically random session IDs
- [ ] Validate destination addresses
- [ ] Implement session timeouts
- [ ] Log authentication failures
- [ ] Rate limit connections
- [ ] Validate all input

### Client Implementation

- [ ] Support TLS
- [ ] Verify server certificates
- [ ] Handle authentication challenges
- [ ] Limit redirect following
- [ ] Validate server responses
- [ ] Implement timeouts

---

## RTSP vs RTSPS

| Feature | RTSP | RTSPS |
|---------|------|-------|
| Port | 554 | 322 |
| Transport | TCP/UDP | TLS over TCP |
| Encryption | None | Full |
| Authentication | Basic/Digest | Mutual TLS + Basic/Digest |

### RTSPS URL

```
rtsps://secure.server.com/movie
```

---

## Summary

| Threat | Mitigation |
|--------|------------|
| Eavesdropping | TLS, SRTP |
| Session hijacking | Random IDs, TLS |
| DoS (amplification) | Source validation |
| DoS (resource) | Limits, timeouts |
| Credential theft | Digest auth, TLS |
| Injection | Input validation |

---

> [Back to Index](00-index.md) | [Previous: Syntax](13-syntax.md) | [Next: State Machines](A-state-machines.md)
