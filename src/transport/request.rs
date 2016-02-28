use hyper;
use serde;

// use Error;
// use error::TransportErrorKind;
// use hyper;
// use serde;
// use serde_json;
// use std;
// use std::io::prelude::*;
//
// #[derive(Debug, PartialEq)]
// pub struct Request {
//     method: hyper::method::Method,
//     path: Vec<String>,
//     query: std::collections::HashMap<String, String>,
//     headers: hyper::header::Headers,
//     body: Body,
// }
//
// impl RequestBuilder {
// }
//
// #[derive(Debug, PartialEq)]
// enum Body {
//     None,
//     Blob(Vec<u8>),
//     Json(JsonBytes),
// }
//
// impl Body {
//     fn into_bytes(self) -> Vec<u8> {}
// }
//
// #[derive(Debug)]
// struct JsonBytes(Vec<u8>);
//
// impl PartialEq for JsonBytes {
//     fn eq(&self, other: &Self) -> bool {
//
//         let &JsonBytes(ref ours) = self;
//         let &JsonBytes(ref theirs) = other;
//
//         if ours.is_empty() && theirs.is_empty() {
//             return true;
//         } else if ours.is_empty() || theirs.is_empty() {
//             return false;
//         }
//
//         let ours: serde_json::Value = serde_json::from_reader(&ours[..]).unwrap();
//         let theirs: serde_json::Value = serde_json::from_reader(&theirs[..]).unwrap();
//
//         ours == theirs
//     }
// }
//
// #[cfg(test)]
// mod tests {
//
//     use Error;
//     use hyper;
//     use serde_json;
//     use std;
//     use super::{Body, JsonBytes, Request, RequestBuilder};
//
//     #[test]
//     fn json_bytes_eq_both_empty() {
//         let a = JsonBytes(Vec::new());
//         let b = JsonBytes(Vec::new());
//         assert_eq!(a, b);
//     }
//
//     #[test]
//     fn json_bytes_eq_one_empty() {
//         let a = JsonBytes(Vec::new());
//         let b = JsonBytes(vec!['{' as u8, '}' as u8]);
//         assert!(a != b);
//         assert!(b != a);
//     }
//
//     #[test]
//     fn json_bytes_eq_nonempty_same() {
//         let a = JsonBytes(String::from("[42, 17]").into());
//         let b = JsonBytes(String::from("[42, 17]").into());
//         assert_eq!(a, b);
//     }
//
//     #[test]
//     fn json_bytes_eq_nonempty_different() {
//         let a = JsonBytes(String::from("[42, 17]").into());
//         let b = JsonBytes(String::from(r#"{"foo": 42}"#).into());
//         assert!(a != b);
//     }
//
//     #[test]
//     fn request_builder_default() {
//
//         let path = vec![String::from("foo"), String::from("bar")];
//
//         let expected = Request {
//             method: hyper::Post,
//             path: path.clone(),
//             query: std::collections::HashMap::new(),
//             headers: hyper::header::Headers::new(),
//             body: Body::None,
//         };
//
//         let got = RequestBuilder::new(hyper::Post, path).unwrap();
//         assert_eq!(expected, got);
//     }
//
//     #[test]
//     fn response_builder_default() {
//
//         let expected = Response {
//             status_code: hyper::BadRequest,
//             headers: hyper::header::Headers::new(),
//             body: Body::None,
//         };
//
//         let got = ResponseBuilder::new(hyper::BadRequest).unwrap();
//         assert_eq!(expected, got);
//     }
//
//     #[test]
//     fn response_builder_with_json_body() {
//
//         let expected = Response {
//             status_code: hyper::Ok,
//             headers: {
//                 let mut headers = hyper::header::Headers::new();
//                 headers.set(hyper::header::ContentType(mime!(Application / Json)));
//                 headers
//             },
//             body: Body::Json(JsonBytes(serde_json::to_vec(&serde_json::builder::ObjectBuilder::new()
//                                           .insert("foo", 42)
//                                           .unwrap())
//                       .unwrap())),
//         };
//
//         let got = ResponseBuilder::new(hyper::Ok)
//                       .with_json_body(&serde_json::builder::ObjectBuilder::new()
//                                            .insert("foo", 42)
//                                            .unwrap())
//                       .unwrap();
//
//         assert_eq!(expected, got);
//     }
//
//     #[test]
//     fn response_decode_json_body_ok() {
//
//         let body = serde_json::builder::ObjectBuilder::new()
//                        .insert("foo", 42)
//                        .unwrap();
//
//         let response = ResponseBuilder::new(hyper::Ok)
//                            .with_json_body(&body)
//                            .unwrap();
//
//         let expected = body;
//         let got: serde_json::Value = response.decode_json_body().unwrap();
//         assert_eq!(expected, got);
//     }
//
//     #[test]
//     fn response_decode_json_body_nok_no_content_type() {
//
//         let response = ResponseBuilder::new(hyper::Ok).unwrap();
//         let got = response.decode_json_body::<serde_json::Value>();
//
//         match got {
//             Err(Error::ResponseNotJson(None)) => (),
//             x @ _ => unexpected_result!(x),
//         }
//     }
//
//     #[test]
//     fn response_decode_json_body_nok_content_type_not_json() {
//
//         use hyper::header::ContentType;
//         use mime::{Mime, TopLevel, SubLevel};
//
//         let response = {
//             let mut response = ResponseBuilder::new(hyper::Ok).unwrap();
//             response.headers.set(ContentType(mime!(Text / Plain)));
//             response
//         };
//
//         let got = response.decode_json_body::<serde_json::Value>();
//
//         match got {
//             Err(Error::ResponseNotJson(Some(Mime(TopLevel::Text, SubLevel::Plain, _)))) => (),
//             x @ _ => unexpected_result!(x),
//         }
//     }
// }
