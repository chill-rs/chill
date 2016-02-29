use hyper;
use mime;
use serde;
use serde_json;
use std;
use transport::Response;
use url;
use uuid;

#[derive(Debug)]
pub enum Error {
    #[doc(hidden)]
    ChannelReceive {
        cause: std::sync::mpsc::RecvError,
        description: &'static str,
    },

    #[doc(hidden)]
    ContentNotAnObject,

    DatabaseExists(ErrorResponse),

    DocumentConflict(ErrorResponse),

    #[doc(hidden)]
    DocumentIsDeleted,

    #[doc(hidden)]
    Io {
        cause: std::io::Error,
        description: &'static str,
    },

    #[doc(hidden)]
    JsonDecode {
        cause: serde_json::Error,
    },

    #[doc(hidden)]
    JsonEncode {
        cause: serde_json::Error,
    },

    #[doc(hidden)]
    Mock {
        extra_description: String,
    },

    #[doc(hidden)]
    NoContentBecauseDeleted,

    NotFound(ErrorResponse),

    #[doc(hidden)]
    ResponseNotJson(Option<mime::Mime>),

    #[doc(hidden)]
    RevisionParse {
        kind: RevisionParseErrorKind,
    },

    #[doc(hidden)]
    ServerResponse {
        status_code: hyper::status::StatusCode,
        error_response: Option<ErrorResponse>,
    },

    #[doc(hidden)]
    Transport {
        kind: TransportErrorKind,
    },

    Unauthorized(ErrorResponse),

    #[doc(hidden)]
    UrlNotSchemeRelative,

    #[doc(hidden)]
    UrlParse {
        cause: url::ParseError,
    },
}

impl Error {
    #[doc(hidden)]
    pub fn server_response(response: Response) -> Self {

        let status_code = response.status_code();

        let error_response = match response.decode_json_body() {
            Ok(x) => Some(x),
            Err(Error::JsonDecode { .. }) => None,
            Err(e) => {
                return e;
            }
        };

        Error::ServerResponse {
            status_code: status_code,
            error_response: error_response,
        }
    }

    #[doc(hidden)]
    pub fn database_exists(response: Response) -> Self {
        match response.decode_json_body() {
            Ok(x) => Error::DatabaseExists(x),
            Err(x) => x,
        }
    }

    #[doc(hidden)]
    pub fn document_conflict(response: Response) -> Self {
        match response.decode_json_body() {
            Ok(x) => Error::DocumentConflict(x),
            Err(x) => x,
        }
    }

    #[doc(hidden)]
    pub fn not_found(response: Response) -> Self {
        match response.decode_json_body() {
            Ok(x) => Error::NotFound(x),
            Err(x) => x,
        }
    }

    #[doc(hidden)]
    pub fn unauthorized(response: Response) -> Self {
        match response.decode_json_body() {
            Ok(x) => Error::Unauthorized(x),
            Err(x) => x,
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        use Error::*;
        match self {
            &ChannelReceive { description, .. } => description,
            &ContentNotAnObject => "Document content is not a JSON object",
            &DatabaseExists(..) => "The database already exists",
            &DocumentConflict(..) => "A conflicting document with the same id exists",
            &DocumentIsDeleted => "The document is deleted",
            &Io { description, .. } => description,
            &JsonDecode { .. } => "An error occurred while decoding JSON",
            &JsonEncode { .. } => "An error occurred while encoding JSON",
            &Mock { .. } => "A error occurred while test-mocking",
            &NoContentBecauseDeleted => "The document is deleted and thus has no content",
            &NotFound(..) => "The resource cannot be found",
            &ResponseNotJson(Some(..)) => "The response has non-JSON content",
            &ResponseNotJson(None) => "The response content has no type",
            &RevisionParse { .. } => "The revision is badly formatted",
            &ServerResponse { ref status_code, .. } => {
                match status_code.class() {
                    hyper::status::StatusClass::ClientError |
                    hyper::status::StatusClass::ServerError => {
                        "The CouchDB server responded with an error"
                    }
                    _ => "The CouchDB server responded with an unexpected status",
                }
            }
            &Transport { .. } => "An HTTP transport error occurred",
            &Unauthorized(..) => "The CouchDB client has insufficient privilege",
            &UrlNotSchemeRelative => "The URL is not scheme relative",
            &UrlParse { .. } => "The URL is badly formatted",
        }
    }

    fn cause(&self) -> Option<&std::error::Error> {
        use Error::*;
        match self {
            &ChannelReceive { ref cause, .. } => Some(cause),
            &ContentNotAnObject => None,
            &DatabaseExists(..) => None,
            &DocumentConflict(..) => None,
            &DocumentIsDeleted => None,
            &Io { ref cause, .. } => Some(cause),
            &JsonDecode { ref cause } => Some(cause),
            &JsonEncode { ref cause } => Some(cause),
            &Mock { .. } => None,
            &NoContentBecauseDeleted => None,
            &NotFound(..) => None,
            &ResponseNotJson(..) => None,
            &RevisionParse { ref kind } => kind.cause(),
            &ServerResponse { .. } => None,
            &Transport { ref kind } => kind.cause(),
            &Unauthorized(..) => None,
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
            &ChannelReceive { ref cause, description } => write!(f, "{}: {}", description, cause),
            &ContentNotAnObject => write!(f, "{}", description),
            &DatabaseExists(ref error_response) => write!(f, "{}: {}", description, error_response),
            &DocumentConflict(ref error_response) => {
                write!(f, "{}: {}", description, error_response)
            }
            &DocumentIsDeleted => write!(f, "{}", description),
            &Io { ref cause, description } => write!(f, "{}: {}", description, cause),
            &JsonDecode { ref cause } => write!(f, "{}: {}", description, cause),
            &JsonEncode { ref cause } => write!(f, "{}: {}", description, cause),
            &Mock { ref extra_description } => write!(f, "{}: {}", description, extra_description),
            &NoContentBecauseDeleted => write!(f, "{}", description),
            &NotFound(ref error_response) => write!(f, "{}: {}", description, error_response),
            &ResponseNotJson(Some(ref content_type)) => {
                write!(f, "{}: Content type is {}", description, content_type)
            }
            &ResponseNotJson(None) => write!(f, "{}", description),
            &RevisionParse { ref kind } => write!(f, "{}: {}", description, kind),
            &ServerResponse { ref status_code, ref error_response } => {
                try!(write!(f, "{} ({}", description, status_code));
                try!(match status_code.canonical_reason() {
                    None => write!(f, ")"),
                    Some(reason) => write!(f, ": {})", reason),
                });
                if let &Some(ref error_response) = error_response {
                    try!(write!(f, ": {}", error_response));
                }
                Ok(())
            }
            &Transport { ref kind } => write!(f, "{}: {}", description, kind),
            &Unauthorized(ref error_response) => write!(f, "{}: {}", description, error_response),
            &UrlNotSchemeRelative => write!(f, "{}", description),
            &UrlParse { ref cause } => write!(f, "{}: {}", description, cause),
        }
    }
}

#[derive(Debug)]
pub enum RevisionParseErrorKind {
    DigestNotAllHex,
    DigestParse(uuid::ParseError),
    NumberParse(std::num::ParseIntError),
    TooFewParts,
    ZeroSequenceNumber,
}

impl RevisionParseErrorKind {
    fn cause(&self) -> Option<&std::error::Error> {
        use self::RevisionParseErrorKind::*;
        match self {
            &DigestNotAllHex => None,
            &DigestParse(ref cause) => Some(cause),
            &NumberParse(ref cause) => Some(cause),
            &TooFewParts => None,
            &ZeroSequenceNumber => None,
        }
    }
}

impl std::fmt::Display for RevisionParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        use self::RevisionParseErrorKind::*;
        match self {
            &DigestNotAllHex => {
                write!(f,
                       "Digest part contains one or more non-hexadecimal characters")
            }
            &DigestParse(ref cause) => write!(f, "The digest part is invalid: {}", cause),
            &NumberParse(ref cause) => write!(f, "The number part is invalid: {}", cause),
            &TooFewParts => write!(f, "Too few parts, missing number part and/or digest part"),
            &ZeroSequenceNumber => write!(f, "The number part is zero"),
        }
    }
}

#[derive(Debug)]
pub enum TransportErrorKind {
    Hyper(hyper::Error),
    Io(std::io::Error),
}

impl TransportErrorKind {
    fn cause(&self) -> Option<&std::error::Error> {
        use self::TransportErrorKind::*;
        match self {
            &Hyper(ref cause) => Some(cause),
            &Io(ref cause) => Some(cause),
        }
    }
}

impl std::fmt::Display for TransportErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        use self::TransportErrorKind::*;
        match self {
            &Hyper(ref cause) => cause.fmt(f),
            &Io(ref cause) => cause.fmt(f),
        }
    }
}

/// Contains error information returned from the CouchDB server when an error
/// occurs while processing the client's request.
#[derive(Clone, Debug, Hash, PartialEq, PartialOrd)]
pub struct ErrorResponse {
    error: String,
    reason: String,
}

impl ErrorResponse {
    #[doc(hidden)]
    pub fn new<T, U>(error: T, reason: U) -> Self
        where T: Into<String>,
              U: Into<String>
    {
        ErrorResponse {
            error: error.into(),
            reason: reason.into(),
        }
    }

    /// Returns the high-level name of the error—e.g., “file_exists”.
    pub fn error(&self) -> &String {
        &self.error
    }

    /// Returns the low-level description of the error—e.g., “The database could
    /// not be created, the file already exists.”
    pub fn reason(&self) -> &String {
        &self.reason
    }
}

impl std::fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{}: {}", self.error, self.reason)
    }
}

impl serde::Deserialize for ErrorResponse {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: serde::Deserializer
    {
        enum Field {
            Error,
            Reason,
        }

        impl serde::Deserialize for Field {
            fn deserialize<D>(deserializer: &mut D) -> Result<Field, D::Error>
                where D: serde::Deserializer
            {
                struct Visitor;

                impl serde::de::Visitor for Visitor {
                    type Value = Field;

                    fn visit_str<E>(&mut self, value: &str) -> Result<Field, E>
                        where E: serde::de::Error
                    {
                        match value {
                            "error" => Ok(Field::Error),
                            "reason" => Ok(Field::Reason),
                            _ => Err(E::unknown_field(value)),
                        }
                    }
                }

                deserializer.deserialize(Visitor)
            }
        }

        struct Visitor;

        impl serde::de::Visitor for Visitor {
            type Value = ErrorResponse;

            fn visit_map<V>(&mut self, mut visitor: V) -> Result<Self::Value, V::Error>
                where V: serde::de::MapVisitor
            {
                let mut error = None;
                let mut reason = None;
                loop {
                    match try!(visitor.visit_key()) {
                        Some(Field::Error) => {
                            error = Some(try!(visitor.visit_value()));
                        }
                        Some(Field::Reason) => {
                            reason = Some(try!(visitor.visit_value()));
                        }
                        None => {
                            break;
                        }
                    }
                }

                try!(visitor.end());

                let x = ErrorResponse {
                    error: match error {
                        Some(x) => x,
                        None => try!(visitor.missing_field("error")),
                    },
                    reason: match reason {
                        Some(x) => x,
                        None => try!(visitor.missing_field("reason")),
                    },
                };

                Ok(x)
            }
        }

        static FIELDS: &'static [&'static str] = &["error", "reason"];
        deserializer.deserialize_struct("ErrorResponse", FIELDS, Visitor)
    }
}

#[cfg(test)]
mod tests {

    use serde_json;
    use super::ErrorResponse;

    #[test]
    fn error_response_display() {
        let source = ErrorResponse {
            error: "file_exists".to_string(),
            reason: "The database could not be created, the file already exists.".to_string(),
        };
        let got = format!("{}", source);
        let error_position = got.find("file_exists").unwrap();
        let reason_position = got.find("The database could not be created, the file already \
                                        exists.")
                                 .unwrap();
        assert!(error_position < reason_position);
    }

    #[test]
    fn error_response_deserialize_ok_with_all_fields() {
        let expected = ErrorResponse {
            error: "foo".to_string(),
            reason: "bar".to_string(),
        };
        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("error", "foo")
                         .insert("reason", "bar")
                         .unwrap();
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str(&source).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn error_response_deserialize_with_with_no_error_field() {
        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("reason", "foo")
                         .unwrap();
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<ErrorResponse>(&source);
        expect_json_error_missing_field!(got, "error");
    }

    #[test]
    fn error_response_deserialize_nok_with_no_reason_field() {
        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("error", "foo")
                         .unwrap();
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<ErrorResponse>(&source);
        expect_json_error_missing_field!(got, "reason");
    }
}
