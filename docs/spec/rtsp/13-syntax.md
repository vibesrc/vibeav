# Section 15: Syntax (ABNF)

> [Back to Index](00-index.md) | [Previous: Examples](12-examples.md) | [Next: Security](14-security.md)

## 15.1 Base Syntax

RTSP syntax defined in ABNF (RFC 2234).

---

## Core ABNF Rules

```abnf
ALPHA     = %x41-5A / %x61-7A   ; A-Z / a-z
DIGIT     = %x30-39             ; 0-9
HEXDIG    = DIGIT / "A" / "B" / "C" / "D" / "E" / "F"
             / "a" / "b" / "c" / "d" / "e" / "f"
SP        = %x20                ; space
HTAB      = %x09                ; horizontal tab
CR        = %x0D                ; carriage return
LF        = %x0A                ; line feed
CRLF      = CR LF               ; standard line ending
CHAR      = %x01-7F             ; any 7-bit US-ASCII character
CTL       = %x00-1F / %x7F      ; control characters
```

---

## RTSP-Specific Constructs

### Version

```abnf
RTSP-Version = "RTSP" "/" 1*DIGIT "." 1*DIGIT
```

Example: `RTSP/1.0`

### URL

```abnf
rtsp-URI     = rtsp-scheme "://" host [":" port] [abs-path ["?" query]]
rtsp-scheme  = "rtsp" / "rtspu"
host         = <host from RFC 2396>
port         = 1*5DIGIT
abs-path     = "/" *segment
segment      = *pchar
pchar        = unreserved / escaped / ":" / "@" / "&" / "=" / "+"
query        = *uric
```

### Request

```abnf
Request       = Request-Line
                *(general-header | request-header | entity-header) CRLF
                CRLF
                [message-body]

Request-Line  = Method SP Request-URI SP RTSP-Version CRLF

Method        = "DESCRIBE" / "ANNOUNCE" / "GET_PARAMETER"
              / "OPTIONS" / "PAUSE" / "PLAY" / "RECORD"
              / "REDIRECT" / "SETUP" / "SET_PARAMETER"
              / "TEARDOWN" / extension-method

extension-method = token
Request-URI   = "*" / absolute-URI
```

### Response

```abnf
Response      = Status-Line
                *(general-header | response-header | entity-header) CRLF
                CRLF
                [message-body]

Status-Line   = RTSP-Version SP Status-Code SP Reason-Phrase CRLF
Status-Code   = 3DIGIT
Reason-Phrase = *<TEXT, excluding CR, LF>
```

---

## Header Syntax

### Generic Header

```abnf
message-header = field-name ":" [field-value]
field-name     = token
field-value    = *(field-content | LWS)
field-content  = <the octets making up the field-value>
LWS            = [CRLF] 1*(SP | HTAB)
```

### Token

```abnf
token      = 1*<any CHAR except CTLs or separators>
separators = "(" / ")" / "<" / ">" / "@"
           / "," / ";" / ":" / "\" / <">
           / "/" / "[" / "]" / "?" / "="
           / "{" / "}" / SP / HTAB
```

---

## Time Range Syntax

### NPT (Normal Play Time)

```abnf
npt-range   = "npt" "=" npt-time "-" [npt-time]
npt-time    = "now" / npt-sec / npt-hhmmss
npt-sec     = 1*DIGIT ["." 1*DIGIT]
npt-hhmmss  = 1*DIGIT ":" 2DIGIT ":" 2DIGIT ["." 1*DIGIT]
```

Examples:
- `npt=0-` (beginning to end)
- `npt=10.5-20` (10.5s to 20s)
- `npt=1:05:00-2:00:00` (1h5m to 2h)

### SMPTE

```abnf
smpte-range = smpte-type "=" smpte-time "-" [smpte-time]
smpte-type  = "smpte" / "smpte-30-drop" / "smpte-25"
smpte-time  = 1*2DIGIT ":" 1*2DIGIT ":" 1*2DIGIT
              [":" 1*2DIGIT ["." 1*2DIGIT]]
```

Example: `smpte=0:10:20:15-0:20:00`

### UTC (Absolute Time)

```abnf
utc-range   = "clock" "=" utc-time "-" [utc-time]
utc-time    = utc-date "T" utc-time-val "Z"
utc-date    = 8DIGIT                    ; YYYYMMDD
utc-time-val = 6DIGIT ["." 1*DIGIT]     ; HHMMSS[.frac]
```

Example: `clock=19961108T143720Z-19961108T153720Z`

---

## Transport Header Syntax

```abnf
Transport        = "Transport" ":" 1#transport-spec
transport-spec   = transport-protocol "/" profile ["/" lower-transport]
                   *parameter

transport-protocol = "RTP"
profile           = "AVP"
lower-transport   = "TCP" / "UDP"

parameter         = ("unicast" / "multicast")
                  / ";" "destination" ["=" address]
                  / ";" "interleaved" "=" channel ["-" channel]
                  / ";" "append"
                  / ";" "ttl" "=" ttl
                  / ";" "layers" "=" 1*DIGIT
                  / ";" "port" "=" port ["-" port]
                  / ";" "client_port" "=" port ["-" port]
                  / ";" "server_port" "=" port ["-" port]
                  / ";" "ssrc" "=" ssrc
                  / ";" "mode" "=" <"> 1#mode <">

ttl               = 1*3DIGIT        ; 0-255
port              = 1*5DIGIT        ; 0-65535
ssrc              = 8HEXDIG
channel           = 1*3DIGIT        ; 0-255
address           = host
mode              = "PLAY" / "RECORD"
```

---

## Session Header Syntax

```abnf
Session = "Session" ":" session-id [";" "timeout" "=" delta-seconds]
session-id = 1*(ALPHA / DIGIT / safe)
safe = "$" / "-" / "_" / "." / "+"
delta-seconds = 1*DIGIT
```

Example: `Session: 12345678;timeout=60`

---

## Range Header Syntax

```abnf
Range = "Range" ":" 1#range-spec
range-spec = npt-range / smpte-range / utc-range
```

---

## RTP-Info Header Syntax

```abnf
RTP-Info      = "RTP-Info" ":" 1#stream-url
stream-url    = "url" "=" quoted-url
                [";" "seq" "=" 1*DIGIT]
                [";" "rtptime" "=" 1*DIGIT]
quoted-url    = <"> absolute-URI <"> / absolute-URI
```

Example:
```
RTP-Info: url=rtsp://server/movie/trackID=1;seq=12345;rtptime=3450012,
          url=rtsp://server/movie/trackID=2;seq=54321;rtptime=2876543
```

---

## Interleaved Binary Data

```abnf
embedded-binary = "$" channel data-length data
channel         = OCTET           ; 1 byte channel number
data-length     = 2OCTET          ; 16-bit big-endian length
data            = *OCTET          ; payload
```

Structure:
```
+--------+--------+--------+--------+--------+...
|  '$'   |channel |length-hi|length-lo| data
| (0x24) |        |                   |
+--------+--------+--------+--------+--------+...
```

---

## Quoted Strings

```abnf
quoted-string = <"> *(qdtext | quoted-pair) <">
qdtext        = <any TEXT except <">>
quoted-pair   = "\" CHAR
```

---

## Comments

```abnf
comment = "(" *(ctext | quoted-pair | comment) ")"
ctext   = <any TEXT excluding "(" and ")">
```

---

## Useful Patterns

### Comma-Separated Lists

```abnf
1#element = element *("," element)
#element  = [1#element]
```

### Optional Elements

```abnf
[element] = *1element
```

### Repetition

```abnf
*element  = 0 or more
1*element = 1 or more
3*element = 3 or more
*5element = 0 to 5
3*5element = 3 to 5
```

---

## Implementation Notes

1. **Case Sensitivity**
   - Methods: CASE-SENSITIVE
   - Header names: case-insensitive
   - Header values: depends on header

2. **Whitespace**
   - LWS between tokens generally allowed
   - CRLF required at end of lines
   - Single SP between Request-Line elements

3. **Line Continuation**
   - Header values can span lines
   - Continue with SP or HTAB at start of next line

---

> [Back to Index](00-index.md) | [Previous: Examples](12-examples.md) | [Next: Security](14-security.md)
