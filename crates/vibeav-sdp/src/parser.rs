//! SDP line parser.
//!
//! Parses SDP text into a Session structure.

use crate::error::SdpError;
use crate::media::{Fmtp, MediaDescription, RtpMap};
use crate::session::{Connection, Direction, Origin, Range, Session, Timing, SDP_VERSION};

/// Parse an SDP string into a Session.
pub fn parse(sdp: &str) -> Result<Session, SdpError> {
    let lines: Vec<&str> = sdp.lines().collect();
    if lines.is_empty() {
        return Err(SdpError::Empty);
    }

    let mut session = Session::new();
    let mut current_media: Option<MediaDescription> = None;
    let mut seen_version = false;

    for (line_num, line) in lines.iter().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Must be at least "x=" format
        if line.len() < 2 || line.as_bytes().get(1) != Some(&b'=') {
            return Err(SdpError::InvalidLine {
                line: line_num + 1,
                message: format!("invalid format: {}", line),
            });
        }

        let type_char = line.as_bytes()[0] as char;
        let value = &line[2..];

        match type_char {
            'v' => {
                let version: u8 = value.parse().map_err(|_| SdpError::InvalidVersion(255))?;
                if version != SDP_VERSION {
                    return Err(SdpError::InvalidVersion(version));
                }
                session.version = version;
                seen_version = true;
            }
            'o' => {
                if let Some(origin) = Origin::parse(value) {
                    session.origin = Some(origin);
                } else {
                    return Err(SdpError::InvalidOrigin(value.to_string()));
                }
            }
            's' => {
                session.name = value.to_string();
            }
            'i' => {
                if let Some(media) = current_media.as_mut() {
                    media.info = Some(value.to_string());
                } else {
                    session.info = Some(value.to_string());
                }
            }
            'u' => {
                session.uri = Some(value.to_string());
            }
            'e' => {
                session.email = Some(value.to_string());
            }
            'p' => {
                session.phone = Some(value.to_string());
            }
            'c' => {
                if let Some(conn) = Connection::parse(value) {
                    if let Some(media) = current_media.as_mut() {
                        media.connection = Some(conn);
                    } else {
                        session.connection = Some(conn);
                    }
                } else {
                    return Err(SdpError::InvalidConnection(value.to_string()));
                }
            }
            't' => {
                if let Some(timing) = Timing::parse(value) {
                    session.timing = Some(timing);
                } else {
                    return Err(SdpError::InvalidTiming(value.to_string()));
                }
            }
            'm' => {
                // Save previous media description
                if let Some(media) = current_media.take() {
                    session.media.push(media);
                }

                if let Some(media) = MediaDescription::parse_media_line(value) {
                    current_media = Some(media);
                } else {
                    return Err(SdpError::InvalidMedia(value.to_string()));
                }
            }
            'a' => {
                parse_attribute(value, &mut session, &mut current_media)?;
            }
            // Ignore bandwidth (b=), repeat (r=), zone (z=), encryption (k=)
            'b' | 'r' | 'z' | 'k' => {}
            _ => {
                // Unknown line type, ignore per RFC
            }
        }
    }

    // Save last media description
    if let Some(media) = current_media {
        session.media.push(media);
    }

    // Validate required fields
    if !seen_version {
        return Err(SdpError::MissingVersion);
    }
    if session.origin.is_none() {
        return Err(SdpError::MissingOrigin);
    }
    if session.name.is_empty() {
        return Err(SdpError::MissingSessionName);
    }

    Ok(session)
}

/// Parse an attribute line.
fn parse_attribute(
    value: &str,
    session: &mut Session,
    current_media: &mut Option<MediaDescription>,
) -> Result<(), SdpError> {
    // Split into name and optional value
    let (name, attr_value) = if let Some((n, v)) = value.split_once(':') {
        (n, Some(v))
    } else {
        (value, None)
    };

    // Direction attributes (no value)
    if let Some(dir) = Direction::from_attr(name) {
        if let Some(media) = current_media.as_mut() {
            media.direction = Some(dir);
        } else {
            session.direction = dir;
        }
        return Ok(());
    }

    match name {
        "control" => {
            let url = attr_value.ok_or_else(|| SdpError::InvalidAttribute("empty control".into()))?;
            if let Some(media) = current_media.as_mut() {
                media.control = Some(url.to_string());
            } else {
                session.control = Some(url.to_string());
            }
        }
        "range" => {
            if let Some(range_str) = attr_value {
                if let Some(range) = Range::parse(range_str) {
                    session.range = Some(range);
                }
            }
        }
        "etag" => {
            if let Some(etag) = attr_value {
                session.etag = Some(etag.to_string());
            }
        }
        "rtpmap" => {
            if let Some(media) = current_media.as_mut() {
                let rtpmap_str =
                    attr_value.ok_or_else(|| SdpError::InvalidRtpmap("empty rtpmap".into()))?;
                if let Some(rtpmap) = RtpMap::parse(rtpmap_str) {
                    media.rtpmap.push(rtpmap);
                } else {
                    return Err(SdpError::InvalidRtpmap(rtpmap_str.to_string()));
                }
            }
        }
        "fmtp" => {
            if let Some(media) = current_media.as_mut() {
                let fmtp_str =
                    attr_value.ok_or_else(|| SdpError::InvalidFmtp("empty fmtp".into()))?;
                if let Some(fmtp) = Fmtp::parse(fmtp_str) {
                    media.fmtp.push(fmtp);
                } else {
                    return Err(SdpError::InvalidFmtp(fmtp_str.to_string()));
                }
            }
        }
        "framerate" => {
            if let Some(media) = current_media.as_mut() {
                if let Some(fr_str) = attr_value {
                    if let Ok(fr) = fr_str.parse() {
                        media.framerate = Some(fr);
                    }
                }
            }
        }
        _ => {
            // Store unknown attributes
            if let Some(media) = current_media.as_mut() {
                media.attributes.push((name.to_string(), attr_value.map(|s| s.to_string())));
            } else {
                session.attributes.push((name.to_string(), attr_value.map(|s| s.to_string())));
            }
        }
    }

    Ok(())
}

/// Resolve a relative control URL against a base URL.
pub fn resolve_control_url(base_url: &str, control: &str) -> String {
    // Asterisk means use base URL as-is
    if control == "*" {
        return base_url.to_string();
    }

    // Already absolute
    if control.starts_with("rtsp://") || control.starts_with("rtsps://") {
        return control.to_string();
    }

    // Relative URL - append to base
    let base = base_url.trim_end_matches('/');
    format!("{}/{}", base, control)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_control_asterisk() {
        assert_eq!(
            resolve_control_url("rtsp://example.com/movie/", "*"),
            "rtsp://example.com/movie/"
        );
    }

    #[test]
    fn test_resolve_control_absolute() {
        assert_eq!(
            resolve_control_url("rtsp://example.com/movie/", "rtsp://other.com/stream"),
            "rtsp://other.com/stream"
        );
    }

    #[test]
    fn test_resolve_control_relative() {
        assert_eq!(
            resolve_control_url("rtsp://example.com/movie/", "trackID=1"),
            "rtsp://example.com/movie/trackID=1"
        );
    }
}
