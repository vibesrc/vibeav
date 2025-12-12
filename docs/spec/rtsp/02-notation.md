# Section 2: Notational Conventions

> [Back to Index](00-index.md) | [Previous: Introduction](01-introduction.md) | [Next: Protocol Parameters](03-protocol-parameters.md)

## ABNF (Augmented Backus-Naur Form)

RTSP uses ABNF as defined in RFC 2234 for syntax specifications. This is the same notation used in HTTP/1.1.

### Basic ABNF Operators

| Operator | Meaning | Example |
|----------|---------|---------|
| `=` | Definition | `rule = definition` |
| `/` | Alternative | `a / b` (a or b) |
| `*` | Repetition | `*element` (0 or more) |
| `1*` | One or more | `1*DIGIT` (at least one digit) |
| `[ ]` | Optional | `[element]` (0 or 1) |
| `( )` | Grouping | `(a b) / c` |
| `" "` | Literal string | `"RTSP"` (case insensitive) |
| `<>` | Prose description | `<any character>` |

### Common ABNF Primitives

```abnf
ALPHA   = %x41-5A / %x61-7A    ; A-Z / a-z
DIGIT   = %x30-39              ; 0-9
HEXDIG  = DIGIT / "A" / "B" / "C" / "D" / "E" / "F"
SP      = %x20                 ; space
HTAB    = %x09                 ; horizontal tab
CRLF    = %x0D %x0A            ; carriage return + line feed
LWS     = [CRLF] 1*(SP / HTAB) ; linear whitespace
```

### HTTP/1.1 Reference

Many RTSP constructs are defined "as in HTTP/1.1". References like `[H4.1]` mean:
- `H` = HTTP/1.1 specification (RFC 2068)
- `4.1` = Section number

For example, `[H10]` refers to HTTP/1.1 Section 10 (Status Code Definitions).

---

## Key Syntax Elements

These are defined fully in [Syntax](13-syntax.md). For RTSP-specific elements like URLs and timestamps, see [Protocol Parameters](03-protocol-parameters.md).

### Basic Tokens

```abnf
token       = 1*<any CHAR except CTLs or separators>
separators  = "(" / ")" / "<" / ">" / "@"
            / "," / ";" / ":" / "\" / <">
            / "/" / "[" / "]" / "?" / "="
            / "{" / "}" / SP / HTAB
```

### RTSP-Specific Elements

```abnf
RTSP-Version = "RTSP" "/" 1*DIGIT "." 1*DIGIT
RTSP-URL     = rtsp_scheme "://" host [":" port] abs_path
rtsp_scheme  = "rtsp" / "rtspu"
```

---

## Case Sensitivity

| Element | Case Sensitivity |
|---------|------------------|
| Method names | Case-SENSITIVE |
| Header field names | Case-INSENSITIVE |
| Header field values | Depends on header |
| Status reason phrases | Case-insensitive (informational) |
| RTSP version | Case-SENSITIVE |

---

> [Back to Index](00-index.md) | [Previous: Introduction](01-introduction.md) | [Next: Protocol Parameters](03-protocol-parameters.md)
