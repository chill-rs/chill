use {NokResponse, std};
use path::PathParseError;
use revision::RevisionParseError;
use std::borrow::Cow;
use std::marker::PhantomData;
use transport::StatusCode;

/// `Error` contains information about an error originating from or propagated
/// by Chill.
#[derive(Debug)]
pub enum Error {
    /// The database already exists.
    DatabaseExists,

    /// The CouchDB server responded with an error or an unknown status.
    NokResponse {
        status_code: StatusCode,
        body: Option<NokResponse>,
        #[doc(hidden)]
        _non_exhaustive: PhantomData<()>,
    },

    #[doc(hidden)]
    PathParse { inner: PathParseError },

    #[doc(hidden)]
    RevisionParse { inner: RevisionParseError },

    #[doc(hidden)]
    Unspecified {
        reason: Cow<'static, str>,
        cause: Option<Box<std::error::Error>>,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let description = std::error::Error::description(self);
        match self {
            &Error::DatabaseExists => write!(f, "{}", description),
            &Error::NokResponse {
                status_code,
                body: Some(ref body),
                ..
            } => write!(
                f,
                "{} (status: {}, error: {}, reason: {:?})",
                description,
                status_code,
                body.error(),
                body.reason()
            ),
            &Error::NokResponse { status_code, .. } => write!(f, "{} (status: {})", description, status_code),
            &Error::PathParse { ref inner } => write!(f, "{}: {}", description, inner),
            &Error::RevisionParse { ref inner } => write!(f, "{}: {}", description, inner),
            &Error::Unspecified {
                cause: Some(ref e),
                ref reason,
            } => write!(f, "{}: {}", reason, e),
            &Error::Unspecified { ref reason, .. } => write!(f, "{}", reason),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match self {
            &Error::DatabaseExists => "The database already exists",
            &Error::NokResponse { .. } => "The CouchDB server responded with an error or an unknown status",
            &Error::PathParse { .. } => "The CouchDB resource path is badly formed",
            &Error::RevisionParse { .. } => "The CouchDB document revision is badly formed",
            &Error::Unspecified { ref reason, .. } => reason.as_ref(),
        }
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match self {
            &Error::Unspecified { cause: Some(ref e), .. } => Some(e.as_ref()),
            _ => None,
        }
    }
}

// We implement From<'static str> and From<String> separately so that we don't
// conflict with From<std::io::Error>.

impl From<&'static str> for Error {
    fn from(reason: &'static str) -> Error {
        Error::Unspecified {
            reason: Cow::Borrowed(reason),
            cause: None,
        }
    }
}

impl From<String> for Error {
    fn from(reason: String) -> Error {
        Error::Unspecified {
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
        Error::Unspecified {
            reason: reason.into(),
            cause: Some(cause.into()),
        }
    }
}

// Implementing From<std::io::Error> allows our Error type to be used with
// Tokio.
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Unspecified {
            reason: Cow::Borrowed("An I/O error occurred"),
            cause: Some(Box::new(e)),
        }
    }
}

impl From<PathParseError> for Error {
    fn from(e: PathParseError) -> Self {
        Error::PathParse { inner: e }
    }
}

impl From<RevisionParseError> for Error {
    fn from(e: RevisionParseError) -> Self {
        Error::RevisionParse { inner: e }
    }
}
