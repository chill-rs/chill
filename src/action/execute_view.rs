use DatabaseName;
use Error;
use IntoViewPath;
use serde;
use std;
use transport::{Action, HyperTransport, RequestOptions, Response, StatusCode, Transport};
use ViewPathRef;
use ViewResponse;
use view::ViewResponseJsonable;

pub struct ExecuteView<'a, T, K, V>
    where K: serde::Deserialize + serde::Serialize,
          T: Transport + 'a,
          V: serde::Deserialize
{
    transport: &'a T,
    view_path: ViewPathRef<'a>,
    phantom_key: std::marker::PhantomData<K>,
    phantom_value: std::marker::PhantomData<V>,
}

impl<'a, K, T, V> ExecuteView<'a, T, K, V>
    where K: serde::Deserialize + serde::Serialize,
          T: Transport + 'a,
          V: serde::Deserialize
{
    #[doc(hidden)]
    pub fn new<P: IntoViewPath<'a>>(transport: &'a T, view_path: P) -> Result<Self, Error> {
        Ok(ExecuteView {
            transport: transport,
            view_path: try!(view_path.into_view_path()),
            phantom_key: std::marker::PhantomData,
            phantom_value: std::marker::PhantomData,
        })
    }
}

impl<'a, K, V> ExecuteView<'a, HyperTransport, K, V>
    where K: serde::Deserialize + serde::Serialize,
          V: serde::Deserialize
{
    pub fn run(self) -> Result<ViewResponse<K, V>, Error> {
        self.transport.exec_sync(self)
    }
}

impl<'a, T, K, V> Action<T> for ExecuteView<'a, T, K, V>
    where K: serde::Deserialize + serde::Serialize,
          T: Transport + 'a,
          V: serde::Deserialize
{
    type Output = ViewResponse<K, V>;
    type State = DatabaseName;

    fn make_request(&mut self) -> Result<(T::Request, Self::State), Error> {
        let options = RequestOptions::new().with_accept_json();
        let db_name = DatabaseName::from(self.view_path.database_name());
        let request = try!(self.transport.get(self.view_path, options));
        Ok((request, db_name))
    }

    fn take_response<R: Response>(response: R,
                                  db_name: Self::State)
                                  -> Result<Self::Output, Error> {
        match response.status_code() {
            StatusCode::Ok => {
                let body: ViewResponseJsonable<K, V> = try!(response.decode_json_body());
                ViewResponse::new_from_decoded(db_name, body)
            }
            StatusCode::NotFound => Err(Error::not_found(response)),
            StatusCode::Unauthorized => Err(Error::unauthorized(response)),
            _ => Err(Error::server_response(response)),
        }
    }
}

#[cfg(test)]
mod tests {

    use DatabaseName;
    use Error;
    use serde_json;
    use super::*;
    use transport::{Action, MockResponse, MockTransport, RequestOptions, StatusCode, Transport};
    use view::ViewResponseBuilder;

    #[test]
    fn make_request_default() {
        let transport = MockTransport::new();

        let expected = ({
            let options = RequestOptions::new().with_accept_json();
            transport.get(vec!["foo", "_design", "bar", "_view", "qux"], options).unwrap()
        },
                        DatabaseName::from("foo"));

        let got = {
            let mut action = ExecuteView::<_, String, i32>::new(&transport,
                                                                "/foo/_design/bar/_view/qux")
                                 .unwrap();
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn take_response_ok_reduced() {

        let response = MockResponse::new(StatusCode::Ok).build_json_body(|x| {
            x.insert_array("rows", |x| {
                x.push_object(|x| {
                    x.insert("key", serde_json::Value::Null)
                     .insert("value", 42)
                })
            })
        });

        let expected = ViewResponseBuilder::new_reduced(42).unwrap();

        let got = ExecuteView::<MockTransport, _, _>::take_response(response,
                                                                    DatabaseName::from("foo"))
                      .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn take_response_ok_unreduced() {

        let response = MockResponse::new(StatusCode::Ok).build_json_body(|x| {
            x.insert("total_rows", 20)
             .insert("offset", 10)
             .insert_array("rows", |x| {
                 x.push_object(|x| {
                      x.insert("key", "Babe Ruth")
                       .insert("value", 714)
                       .insert("id", "babe_ruth")
                  })
                  .push_object(|x| {
                      x.insert("key", "Hank Aaron")
                       .insert("value", 755)
                       .insert("id", "hank_aaron")
                  })
             })
        });

        let expected = ViewResponseBuilder::<String, i32, _>::new_unreduced(20, 10, "baseball")
                           .with_row("Babe Ruth", 714, "babe_ruth")
                           .with_row("Hank Aaron", 755, "hank_aaron")
                           .unwrap();

        let db_name = DatabaseName::from("baseball");
        let got = ExecuteView::<MockTransport, _, _>::take_response(response, db_name).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn take_response_not_found() {
        let error = "not_found";
        let reason = "missing_named_view";
        let response = MockResponse::new(StatusCode::NotFound).build_json_body(|x| {
            x.insert("error", error)
             .insert("reason", reason)
        });
        let db_name = DatabaseName::from("foo");
        match ExecuteView::<MockTransport, String, i32>::take_response(response, db_name) {
            Err(Error::NotFound(ref error_response)) if error == error_response.error() &&
                                                        reason == error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn take_response_unauthorized() {
        let error = "unauthorized";
        let reason = "Authentication required.";
        let response = MockResponse::new(StatusCode::Unauthorized).build_json_body(|x| {
            x.insert("error", error)
             .insert("reason", reason)
        });
        let db_name = DatabaseName::from("foo");
        match ExecuteView::<MockTransport, String, i32>::take_response(response, db_name) {
            Err(Error::Unauthorized(ref error_response)) if error == error_response.error() &&
                                                            reason == error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }
}
