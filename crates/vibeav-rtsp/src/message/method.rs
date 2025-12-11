//! RTSP methods.

use std::fmt;
use std::str::FromStr;

use crate::error::{RtspError, Result};

/// RTSP request methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    /// Query server capabilities.
    Options,
    /// Get presentation description (SDP).
    Describe,
    /// Post or update session description.
    Announce,
    /// Set up transport for a stream.
    Setup,
    /// Start media delivery.
    Play,
    /// Pause media delivery.
    Pause,
    /// Start recording/receiving media.
    Record,
    /// End the session.
    Teardown,
    /// Get parameter value.
    GetParameter,
    /// Set parameter value.
    SetParameter,
    /// Redirect client to another server.
    Redirect,
}

impl Method {
    /// Check if this method is safe (doesn't modify state).
    pub fn is_safe(&self) -> bool {
        matches!(
            self,
            Method::Options | Method::Describe | Method::GetParameter
        )
    }

    /// Check if this method requires a session.
    pub fn requires_session(&self) -> bool {
        matches!(
            self,
            Method::Play
                | Method::Pause
                | Method::Record
                | Method::Teardown
                | Method::GetParameter
                | Method::SetParameter
        )
    }

    /// Get the method name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Method::Options => "OPTIONS",
            Method::Describe => "DESCRIBE",
            Method::Announce => "ANNOUNCE",
            Method::Setup => "SETUP",
            Method::Play => "PLAY",
            Method::Pause => "PAUSE",
            Method::Record => "RECORD",
            Method::Teardown => "TEARDOWN",
            Method::GetParameter => "GET_PARAMETER",
            Method::SetParameter => "SET_PARAMETER",
            Method::Redirect => "REDIRECT",
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Method {
    type Err = RtspError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_uppercase().as_str() {
            "OPTIONS" => Ok(Method::Options),
            "DESCRIBE" => Ok(Method::Describe),
            "ANNOUNCE" => Ok(Method::Announce),
            "SETUP" => Ok(Method::Setup),
            "PLAY" => Ok(Method::Play),
            "PAUSE" => Ok(Method::Pause),
            "RECORD" => Ok(Method::Record),
            "TEARDOWN" => Ok(Method::Teardown),
            "GET_PARAMETER" => Ok(Method::GetParameter),
            "SET_PARAMETER" => Ok(Method::SetParameter),
            "REDIRECT" => Ok(Method::Redirect),
            _ => Err(RtspError::InvalidMethod(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_from_str() {
        assert_eq!("OPTIONS".parse::<Method>().unwrap(), Method::Options);
        assert_eq!("options".parse::<Method>().unwrap(), Method::Options);
        assert_eq!("DESCRIBE".parse::<Method>().unwrap(), Method::Describe);
        assert_eq!("SETUP".parse::<Method>().unwrap(), Method::Setup);
        assert_eq!("PLAY".parse::<Method>().unwrap(), Method::Play);
        assert_eq!("RECORD".parse::<Method>().unwrap(), Method::Record);
        assert!("INVALID".parse::<Method>().is_err());
    }

    #[test]
    fn test_method_display() {
        assert_eq!(Method::Options.to_string(), "OPTIONS");
        assert_eq!(Method::Describe.to_string(), "DESCRIBE");
        assert_eq!(Method::GetParameter.to_string(), "GET_PARAMETER");
    }

    #[test]
    fn test_method_requires_session() {
        assert!(!Method::Options.requires_session());
        assert!(!Method::Describe.requires_session());
        assert!(!Method::Setup.requires_session());
        assert!(Method::Play.requires_session());
        assert!(Method::Teardown.requires_session());
        assert!(Method::Record.requires_session());
    }
}
