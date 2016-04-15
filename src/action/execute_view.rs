use prelude_impl::*;
use serde;
use std;

enum Inclusivity {
    Exclusive,
    Inclusive,
}

pub struct ExecuteView<'a, T, K, V>
    where K: serde::Deserialize + serde::Serialize + 'a,
          T: Transport + 'a,
          V: serde::Deserialize
{
    transport: &'a T,
    view_path: ViewPathRef<'a>,
    phantom_key: std::marker::PhantomData<K>,
    phantom_value: std::marker::PhantomData<V>,
    reduce: Option<bool>,
    start_key: Option<&'a K>,
    end_key: Option<(&'a K, Inclusivity)>,
    descending: Option<bool>,
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
            reduce: None,
            start_key: None,
            end_key: None,
            descending: None,
        })
    }

    /// Modifies the action to explicitly reduce or not reduce the view.
    ///
    /// The `with_reduce` method abstracts CouchDB's `reduce` query parameter.
    /// By default, CouchDB reduces a view if and only if the view contains a
    /// reduction function. Consequently, an application may use this method to
    /// disable reduction of a view that contains a reduction function.
    ///
    pub fn with_reduce(mut self, reduce: bool) -> Self {
        self.reduce = Some(reduce);
        self
    }

    /// Modifies the action to include only records with a key greater than or
    /// equal to a given key.
    ///
    /// The `with_start_key` method abstracts CouchDB's `startkey` query
    /// parameter. By default, the CouchDB server includes all records.
    ///
    pub fn with_start_key(mut self, start_key: &'a K) -> Self {
        self.start_key = Some(start_key);
        self
    }

    /// Modifies the action to include only records with a key less than or
    /// equal to a given key.
    ///
    /// The `with_end_key_inclusive` method abstracts CouchDB's `endkey` query
    /// parameter. By default, the CouchDB server includes all records.
    ///
    pub fn with_end_key_inclusive(mut self, end_key: &'a K) -> Self {
        self.end_key = Some((end_key, Inclusivity::Inclusive));
        self
    }

    /// Modifies the action to include only records with a key less than a given
    /// key.
    ///
    /// The `with_end_key_exclusive` method abstracts CouchDB's `endkey` and
    /// `inclusive_end` query parameters. By default, the CouchDB server
    /// includes all records.
    ///
    pub fn with_end_key_exclusive(mut self, end_key: &'a K) -> Self {
        self.end_key = Some((end_key, Inclusivity::Exclusive));
        self
    }

    /// Modifies the action to retrieve the view rows in descending order.
    ///
    /// The `with_descending` method abstracts CouchDB's `descending` query
    /// parameter. By default, the CouchDB server sends the rows of an unreduced
    /// view in ascending order, sorted by key. Whereas, if the `descending`
    /// query parameter is `true`, then the server sends the rows in reverse
    /// order.
    ///
    /// This method has no effect if the view is reduced.
    ///
    pub fn with_descending(mut self, descending: bool) -> Self {
        self.descending = Some(descending);
        self
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

        let options = match self.reduce {
            None => options,
            Some(value) => options.with_reduce_query(value),
        };

        let options = match self.start_key {
            None => options,
            Some(value) => try!(options.with_start_key(value)),
        };

        let options = match self.end_key {
            None => options,
            Some((key_value, Inclusivity::Inclusive)) => try!(options.with_end_key(key_value)),
            Some((key_value, Inclusivity::Exclusive)) => {
                try!(options.with_end_key(key_value)).with_inclusive_end(false)
            }
        };

        let options = match self.descending {
            None => options,
            Some(value) => options.with_descending_query(value),
        };

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

    use prelude_impl::*;
    use serde_json;
    use super::ExecuteView;

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
    fn make_request_with_descending() {
        let transport = MockTransport::new();

        let expected = ({
            let options = RequestOptions::new().with_descending_query(true).with_accept_json();
            transport.get(vec!["foo", "_design", "bar", "_view", "qux"], options).unwrap()
        },
                        DatabaseName::from("foo"));

        let got = {
            let mut action = ExecuteView::<_, String, i32>::new(&transport,
                                                                "/foo/_design/bar/_view/qux")
                                 .unwrap()
                                 .with_descending(true);
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn make_request_with_end_key_exclusive() {
        let transport = MockTransport::new();

        let end_key = String::from("my key");

        let expected = ({
            let options = RequestOptions::new()
                              .with_end_key(&end_key)
                              .unwrap()
                              .with_inclusive_end(false)
                              .with_accept_json();
            transport.get(vec!["foo", "_design", "bar", "_view", "qux"], options).unwrap()
        },
                        DatabaseName::from("foo"));

        let got = {
            let mut action = ExecuteView::<_, String, i32>::new(&transport,
                                                                "/foo/_design/bar/_view/qux")
                                 .unwrap()
                                 .with_end_key_exclusive(&end_key);
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn make_request_with_end_key_inclusive() {
        let transport = MockTransport::new();

        let end_key = String::from("my key");

        let expected = ({
            let options = RequestOptions::new()
                              .with_end_key(&end_key)
                              .unwrap()
                              .with_accept_json();
            transport.get(vec!["foo", "_design", "bar", "_view", "qux"], options).unwrap()
        },
                        DatabaseName::from("foo"));

        let got = {
            let mut action = ExecuteView::<_, String, i32>::new(&transport,
                                                                "/foo/_design/bar/_view/qux")
                                 .unwrap()
                                 .with_end_key_inclusive(&end_key);
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn make_request_with_reduce() {
        let transport = MockTransport::new();

        let expected = ({
            let options = RequestOptions::new().with_reduce_query(false).with_accept_json();
            transport.get(vec!["foo", "_design", "bar", "_view", "qux"], options).unwrap()
        },
                        DatabaseName::from("foo"));

        let got = {
            let mut action = ExecuteView::<_, String, i32>::new(&transport,
                                                                "/foo/_design/bar/_view/qux")
                                 .unwrap()
                                 .with_reduce(false);
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn make_request_with_start_key() {
        let transport = MockTransport::new();

        let start_key = String::from("my key");

        let expected = ({
            let options = RequestOptions::new()
                              .with_start_key(&start_key)
                              .unwrap()
                              .with_accept_json();
            transport.get(vec!["foo", "_design", "bar", "_view", "qux"], options).unwrap()
        },
                        DatabaseName::from("foo"));

        let got = {
            let mut action = ExecuteView::<_, String, i32>::new(&transport,
                                                                "/foo/_design/bar/_view/qux")
                                 .unwrap()
                                 .with_start_key(&start_key);
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
