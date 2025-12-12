//! SDP (Session Description Protocol) parser for vibeav.
//!
//! Parses SDP as used in RTSP for media session descriptions.
//!
//! # Example
//!
//! ```
//! use vibeav_sdp::{parse, resolve_control_url};
//!
//! let sdp = r#"v=0
//! o=- 1234 1234 IN IP4 192.168.1.1
//! s=Example
//! t=0 0
//! a=control:rtsp://example.com/stream
//! m=video 0 RTP/AVP 96
//! a=rtpmap:96 H264/90000
//! a=control:trackID=1
//! "#;
//!
//! let session = parse(sdp).unwrap();
//! assert_eq!(session.name, "Example");
//! assert_eq!(session.media.len(), 1);
//!
//! let video = &session.media[0];
//! assert_eq!(video.media_type, "video");
//! assert_eq!(video.encoding(), Some("H264"));
//! ```

mod error;
mod media;
mod parser;
mod session;

pub use error::SdpError;
pub use media::{Fmtp, MediaDescription, RtpMap};
pub use parser::{parse, resolve_control_url};
pub use session::{Connection, Direction, Origin, Range, Session, Timing, SDP_VERSION};
