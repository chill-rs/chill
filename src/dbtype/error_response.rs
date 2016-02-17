use serde;
use std;

/// Contains error information returned from CouchDB server if an error occurs
/// while processing the client's request.
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

                deserializer.visit(Visitor)
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
        deserializer.visit_struct("ErrorResponse", FIELDS, Visitor)
    }
}

#[cfg(test)]
mod tests {

    use serde_json;
    use super::ErrorResponse;

    #[test]
    fn display() {
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
    fn deserialize_ok_with_all_fields() {
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
    fn deserialize_with_with_no_error_field() {
        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("reason", "foo")
                         .unwrap();
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<ErrorResponse>(&source);
        expect_json_error_missing_field!(got, "error");
    }

    #[test]
    fn deserialize_nok_with_no_reason_field() {
        let source = serde_json::builder::ObjectBuilder::new()
                         .insert("error", "foo")
                         .unwrap();
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<ErrorResponse>(&source);
        expect_json_error_missing_field!(got, "reason");
    }
}
