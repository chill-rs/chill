use ErrorResponse;
use hyper;
use serde_json;
use std;
use transport::Response;
use url;

#[derive(Debug)]
pub enum Error {
    DatabaseExists(ErrorResponse),

    #[doc(hidden)]
    JsonDecode {
        cause: serde_json::Error,
    },

    #[doc(hidden)]
    JsonEncode {
        cause: serde_json::Error,
    },

    #[doc(hidden)]
    Transport {
        kind: TransportErrorKind,
    },

    Unauthorized(ErrorResponse),

    #[doc(hidden)]
    UnexpectedResponse {
        status_code: hyper::status::StatusCode,
    },

    #[doc(hidden)]
    UrlNotSchemeRelative,

    #[doc(hidden)]
    UrlParse {
        cause: url::ParseError,
    },
}

pub fn database_exists<R>(response: R) -> Error
    where R: Response
{
    let error_response = match response.json_decode_content() {
        Ok(x) => x,
        Err(e) => {
            return e;
        }
    };

    Error::DatabaseExists(error_response)
}

pub fn unauthorized<R>(response: R) -> Error
    where R: Response
{
    let error_response = match response.json_decode_content() {
        Ok(x) => x,
        Err(e) => {
            return e;
        }
    };

    Error::Unauthorized(error_response)
}

pub fn unexpected_response<R>(response: R) -> Error
    where R: Response
{
    Error::UnexpectedResponse { status_code: response.status_code() }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        use Error::*;
        match self {
            &DatabaseExists(..) => "The database already exists",
            &JsonDecode { .. } => "An error occurred while decoding JSON",
            &JsonEncode { .. } => "An error occurred while encoding JSON",
            &Transport { .. } => "An HTTP transport error occurred",
            &Unauthorized(..) => "The CouchDB client has insufficient privilege",
            &UnexpectedResponse { .. } => {
                "The CouchDB server responded with an unexpected status code"
            }
            &UrlNotSchemeRelative => "The URL is not scheme relative",
            &UrlParse { .. } => "The URL is invalid",
        }
    }

    fn cause(&self) -> Option<&std::error::Error> {
        use Error::*;
        match self {
            &DatabaseExists(..) => None,
            &JsonDecode { ref cause } => Some(cause),
            &JsonEncode { ref cause } => Some(cause),
            &Transport { ref kind } => kind.cause(),
            &Unauthorized(..) => None,
            &UnexpectedResponse { .. } => None,
            &UrlNotSchemeRelative => None,
            &UrlParse { ref cause } => Some(cause),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        use Error::*;
        let description = std::error::Error::description(self);
        match self {
            &DatabaseExists(ref error_response) => write!(f, "{}: {}", description, error_response),
            &JsonDecode { ref cause } => write!(f, "{}: {}", description, cause),
            &JsonEncode { ref cause } => write!(f, "{}: {}", description, cause),
            &Transport { ref kind } => write!(f, "{}: {}", description, kind),
            &Unauthorized(ref error_response) => write!(f, "{}: {}", description, error_response),
            &UnexpectedResponse { ref status_code } => {
                write!(f,
                       "{} ({}: {})",
                       description,
                       status_code,
                       match status_code.canonical_reason() {
                           None => "N/a",
                           Some(reason) => reason,
                       })
            }
            &UrlNotSchemeRelative => write!(f, "{}", description),
            &UrlParse { ref cause } => write!(f, "{}: {}", description, cause),
        }
    }
}

#[derive(Debug)]
pub enum TransportErrorKind {
    Hyper(hyper::Error),
}

impl TransportErrorKind {
    fn cause(&self) -> Option<&std::error::Error> {
        use self::TransportErrorKind::*;
        match self {
            &Hyper(ref cause) => Some(cause),
        }
    }
}

impl std::fmt::Display for TransportErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        use self::TransportErrorKind::*;
        match self {
            &Hyper(ref cause) => cause.fmt(f),
        }
    }
}
