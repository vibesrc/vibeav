# Session-Level Fields

## Protocol Version (`v=`)

```
v=0
```

The version of SDP. This memo defines version 0. There is no minor version number.

**Requirements:**
- MUST be the first line
- MUST be exactly `0`

## Origin (`o=`)

```
o=<username> <sess-id> <sess-version> <nettype> <addrtype> <unicast-address>
```

The originator of the session plus session identifier and version.

### Fields

| Field | Description |
|-------|-------------|
| `username` | User's login or `-` if not available. MUST NOT contain spaces. |
| `sess-id` | Numeric string forming globally unique ID (with other fields). NTP timestamp recommended. |
| `sess-version` | Version number for this description. Increase when modified. NTP timestamp recommended. |
| `nettype` | Network type. Currently only `IN` (Internet) defined. |
| `addrtype` | Address type. `IP4` or `IP6`. |
| `unicast-address` | FQDN or IP address of originator. |

### Example

```
o=- 1234567890 1234567891 IN IP4 192.168.1.1
```

### Global Uniqueness

The tuple `(username, sess-id, nettype, addrtype, unicast-address)` forms a globally unique identifier for the session.

### Privacy

For privacy, arbitrary username and private unicast-address MAY be chosen, provided global uniqueness is maintained.

## Session Name (`s=`)

```
s=<session name>
```

The textual session name.

**Requirements:**
- MUST have exactly one `s=` field
- MUST NOT be empty
- If no meaningful name, use `s= ` (single space)
- SHOULD contain ISO 10646 characters (UTF-8)

### Example

```
s=Weekly Team Meeting
```

## Session Information (`i=`)

```
i=<session description>
```

Free-form textual information about the session.

**Requirements:**
- OPTIONAL
- At most one session-level `i=` field
- At most one `i=` field per media description
- Human-readable, not for machine parsing

### Example

```
i=This is a weekly video conference for the engineering team
```

## URI (`u=`)

```
u=<uri>
```

A URI pointing to additional information about the session.

**Requirements:**
- OPTIONAL
- At most one per session
- MUST appear before first media field

### Example

```
u=http://www.example.com/meetings/info.html
```

## Email Address (`e=`)

```
e=<email-address>
```

Contact email for the person responsible for the conference.

**Requirements:**
- OPTIONAL (was required in RFC 2327)
- Multiple allowed
- MUST appear before first media field

### Formats

```
e=j.doe@example.com (Jane Doe)
e=Jane Doe <j.doe@example.com>
e=j.doe@example.com
```

## Phone Number (`p=`)

```
p=<phone-number>
```

Contact phone for the person responsible for the conference.

**Requirements:**
- OPTIONAL (was required in RFC 2327)
- SHOULD be international format with `+` prefix
- Multiple allowed
- MUST appear before first media field

### Formats

```
p=+1 617 555-6011
p=+1-617-555-6011 (Jane Doe)
p=Jane Doe <+1 617 555 6011>
```
