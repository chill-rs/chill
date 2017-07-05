use {NokResponse, std};
use std::borrow::Cow;
use transport::StatusCode;

/// `Error` contains information about an error either originating from or
/// propagated by Chill.
#[derive(Debug)]
pub enum Error {
    /// The database already exists.
    DatabaseExists,

    #[doc(hidden)]
    NokResponse {
        status_code: StatusCode,
        body: Option<NokResponse>,
    },

    #[doc(hidden)]
    Recursive {
        reason: Cow<'static, str>,
        cause: Option<Box<std::error::Error>>,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let d = std::error::Error::description(self);
        match self {
            &Error::DatabaseExists => write!(f, "{}", d),
            &Error::NokResponse {
                status_code,
                body: Some(ref body),
            } => write!(
                f,
                "{} (status: {}, error: {}, reason: {:?})",
                d,
                status_code,
                body.error(),
                body.reason()
            ),
            &Error::NokResponse { status_code, .. } => write!(f, "{} (status: {})", d, status_code),
            &Error::Recursive {
                cause: Some(ref e),
                ref reason,
            } => write!(f, "{}: {}", reason, e),
            &Error::Recursive { ref reason, .. } => write!(f, "{}", reason),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match self {
            &Error::DatabaseExists => "The database already exists",
            &Error::NokResponse { .. } => "The CouchDB server responded with error",
            &Error::Recursive { ref reason, .. } => reason.as_ref(),
        }
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match self {
            &Error::Recursive { cause: Some(ref e), .. } => Some(e.as_ref()),
            _ => None,
        }
    }
}

// We implement From<'static str> and From<String> separately so that we don't
// conflict with From<std::io::Error>.

impl From<&'static str> for Error {
    fn from(reason: &'static str) -> Error {
        Error::Recursive {
            reason: Cow::Borrowed(reason),
            cause: None,
        }
    }
}

impl From<String> for Error {
    fn from(reason: String) -> Error {
        Error::Recursive {
            reason: Cow::Owned(reason),
            cause: None,
        }
    }
}

impl<E, R> From<(R, E)> for Error
where
    E: Into<Box<std::error::Error>>,
    R: Into<Cow<'static, str>>,
{
    fn from((reason, cause): (R, E)) -> Error {
        Error::Recursive {
            reason: reason.into(),
            cause: Some(cause.into()),
        }
    }
}

// Implementing From<std::io::Error> allows our Error type to be used with
// Tokio.
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Recursive {
            reason: Cow::Borrowed("An I/O error occurred"),
            cause: Some(Box::new(e)),
        }
    }
}
