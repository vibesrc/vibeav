# RFC 2326: Real Time Streaming Protocol (RTSP) Specification

> **Source**: RFC 2326 (April 1998)
> **Status**: Proposed Standard (Obsoleted by RFC 7826)
> **Authors**: H. Schulzrinne, A. Rao, R. Lanphier

## Overview

RTSP is an application-level protocol for control over delivery of data with real-time properties. It provides an extensible framework for controlled, on-demand delivery of real-time data such as audio and video.

**Key characteristics:**
- Acts as a "network remote control" for multimedia servers
- Similar to HTTP/1.1 in syntax but stateful
- Controls streams but typically doesn't deliver them (uses RTP)
- Default port: 554

## Document Index

### Core Protocol

| Section | File | Description |
|---------|------|-------------|
| 1 | [01-introduction.md](01-introduction.md) | Purpose, terminology, protocol properties, states |
| 2 | [02-notation.md](02-notation.md) | Notational conventions (ABNF) |
| 3 | [03-protocol-parameters.md](03-protocol-parameters.md) | URLs, session IDs, timestamps, option tags |
| 4-5 | [04-message-format.md](04-message-format.md) | RTSP message structure, general headers |
| 6-7 | [05-request-response.md](05-request-response.md) | Request and response format |
| 8-9 | [06-entity-connections.md](06-entity-connections.md) | Entity body, connections, pipelining |

### Methods & Status Codes

| Section | File | Description |
|---------|------|-------------|
| 10 | [07-methods.md](07-methods.md) | All RTSP method definitions |
| 11 | [08-status-codes.md](08-status-codes.md) | Status code definitions |

### Headers

| Section | File | Description |
|---------|------|-------------|
| 12 | [09-headers.md](09-headers.md) | Header field definitions overview |
| 12.39 | [10-transport-header.md](10-transport-header.md) | Transport header (critical for implementation) |

### Additional Topics

| Section | File | Description |
|---------|------|-------------|
| 13 | [11-caching.md](11-caching.md) | Caching considerations |
| 14 | [12-examples.md](12-examples.md) | Protocol usage examples |
| 15 | [13-syntax.md](13-syntax.md) | ABNF syntax definitions |
| 16 | [14-security.md](14-security.md) | Security considerations |

### Appendices

| Appendix | File | Description |
|----------|------|-------------|
| A | [A-state-machines.md](A-state-machines.md) | Client/server state machines |
| B | [B-rtp-interaction.md](B-rtp-interaction.md) | RTP interaction details |
| C | [C-sdp-usage.md](C-sdp-usage.md) | SDP for RTSP session descriptions |
| D | [D-minimal-implementation.md](D-minimal-implementation.md) | Minimal client/server requirements |

---

## Quick Reference

### Required Methods
- `OPTIONS` - Query server capabilities
- `SETUP` - Establish transport parameters
- `PLAY` - Start media delivery
- `TEARDOWN` - Stop session and free resources

### Recommended Methods
- `DESCRIBE` - Get presentation description (SDP)
- `PAUSE` - Temporarily halt delivery

### Optional Methods
- `ANNOUNCE` - Post/update session description
- `GET_PARAMETER` / `SET_PARAMETER` - Get/set parameters
- `RECORD` - Start recording
- `REDIRECT` - Redirect client to new server

### Common Response Codes

See [Status Codes](08-status-codes.md) for full list.

| Code | Meaning |
|------|---------|
| 200 | OK |
| 454 | Session Not Found |
| 455 | Method Not Valid in This State |
| 461 | Unsupported Transport |

---

## Related Protocols

| Protocol | Documentation | Description |
|----------|---------------|-------------|
| RTP | [RTP Specification](../rtp/00-index.md) | Media data transport |
| RTCP | [RTP/RTCP](../rtp/06-rtcp-overview.md) | Quality feedback |
| SDP | [SDP Usage](C-sdp-usage.md) | Session descriptions |

---

## Implementation Notes

For implementing an RTSP client or server, read these files in order:
1. [01-introduction.md](01-introduction.md) - Understand core concepts
2. [03-protocol-parameters.md](03-protocol-parameters.md) - URL and timestamp formats
3. [07-methods.md](07-methods.md) - Method semantics
4. [10-transport-header.md](10-transport-header.md) - Transport negotiation
5. [A-state-machines.md](A-state-machines.md) - State transitions
6. [C-sdp-usage.md](C-sdp-usage.md) - SDP integration
7. [12-examples.md](12-examples.md) - See it all in action

For RTP media transport, see:
- [RTP Header Format](../rtp/04-rtp-header.md)
- [RTCP Reports](../rtp/07-rtcp-reports.md) - Synchronization info
- [RTP/AVP Profile](../rtp/13-avp-profile.md) - Payload types
