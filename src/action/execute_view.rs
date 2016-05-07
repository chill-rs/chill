//! Defines an action for executing a view.

use {DatabaseName, Error, IntoViewPath, ViewResponse};
use transport::{Action, RequestOptions, Response, StatusCode, Transport};
use transport::production::HyperTransport;
use view::ViewResponseJsonable;
use {serde, std};

enum Inclusivity {
    Exclusive,
    Inclusive,
}

/// Executes a view on the CouchDB server and returns the result.
///
/// Chill executes the view by sending an HTTP request to the CouchDB server to
/// `GET` from or `POST` to the view's path. For more details about executing
/// views, please see the CouchDB documentation.
///
/// # Errors
///
/// The following are _some_ errors that may occur when executing a view.
///
/// <table>
/// <tr>
///  <td><code>Error::NotFound</code></td>
///  <td>The database, design document, or view does not exist.</td>
/// </tr>
/// <tr>
///  <td><code>Error::Unauthorized</code></td>
///  <td>The client lacks permission to execute the view.</td>
/// </tr>
/// </table>
///
/// # Examples
///
/// The following program demonstrates view execution.
///
/// ```
/// extern crate chill;
/// extern crate serde_json;
///
/// let server = chill::testing::FakeServer::new().unwrap();
/// let client = chill::Client::new(server.uri()).unwrap();
///
/// // Create a database and populate it with some documents.
///
/// client.create_database("/baseball").run().unwrap();
///
/// let create_player = |name, home_runs| {
///     client.create_document("/baseball",
///                            &serde_json::builder::ObjectBuilder::new()
///                                 .insert("name", name)
///                                 .insert("home_runs", home_runs)
///                                 .unwrap())
///           .run()
///           .unwrap();
/// };
///
/// create_player("Babe Ruth", 714);
/// create_player("Hank Aaron", 755);
/// create_player("Willie Mays", 660);
///
/// client.create_document("/baseball", {
///           &serde_json::builder::ObjectBuilder::new()
///                .insert_object("views", |x| {
///                    x.insert_object("home_run", |x| {
///                        x.insert("map", r#"function(doc) { emit(doc.home_runs, doc.name); }"#)
///                    })
///                })
///                .unwrap()
///       })
///       .with_document_id("_design/stat")
///       .run()
///       .unwrap();
///
/// // Execute a view to get players with at least 700 home runs.
///
/// let view_response = client.execute_view::<i32, String, _>(
///                               "/baseball/_design/stat/_view/home_run")
///                           .with_descending(true)
///                           .with_end_key_inclusive(&700)
///                           .run()
///                           .unwrap();
///
/// let expected = vec![
///     (755, "Hank Aaron"),
///     (714, "Babe Ruth"),
/// ];
///
/// let got = view_response.as_unreduced()
///                        .expect("View response is not unreduced")
///                        .rows()
///                        .iter()
///                        .map(|x| (*x.key(), x.value().as_ref()))
///                        .collect::<Vec<(i32, &str)>>();
///
/// assert_eq!(expected, got);
/// ```
///
pub struct ExecuteView<'a, T, P, K, V>
    where K: serde::Deserialize + serde::Serialize + 'a,
          P: IntoViewPath,
          T: Transport + 'a,
          V: serde::Deserialize
{
    transport: &'a T,
    view_path: P,
    phantom_key: std::marker::PhantomData<K>,
    phantom_value: std::marker::PhantomData<V>,
    reduce: Option<bool>,
    start_key: Option<&'a K>,
    end_key: Option<(&'a K, Inclusivity)>,
    limit: Option<u64>,
    descending: Option<bool>,
}

impl<'a, K, P, T, V> ExecuteView<'a, T, P, K, V>
    where K: serde::Deserialize + serde::Serialize,
          P: IntoViewPath,
          T: Transport + 'a,
          V: serde::Deserialize
{
    #[doc(hidden)]
    pub fn new(transport: &'a T, view_path: P) -> Self {
        ExecuteView {
            transport: transport,
            view_path: view_path,
            phantom_key: std::marker::PhantomData,
            phantom_value: std::marker::PhantomData,
            reduce: None,
            start_key: None,
            end_key: None,
            limit: None,
            descending: None,
        }
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

    /// Modifies the action to retrieve at most a given number of documents.
    ///
    /// The `with_limit` method abstracts CouchDB's `limit` query parameter. By
    /// default, the CouchDB server sends all rows.
    ///
    /// This method has no effect if the view is reduced.
    ///
    pub fn with_limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
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

impl<'a, K, P, V> ExecuteView<'a, HyperTransport, P, K, V>
    where K: serde::Deserialize + serde::Serialize,
          P: IntoViewPath,
          V: serde::Deserialize
{
    pub fn run(self) -> Result<ViewResponse<K, V>, Error> {
        self.transport.exec_sync(self)
    }
}

impl<'a, K, P, T, V> Action<T> for ExecuteView<'a, T, P, K, V>
    where K: serde::Deserialize + serde::Serialize,
          P: IntoViewPath,
          T: Transport + 'a,
          V: serde::Deserialize
{
    type Output = ViewResponse<K, V>;
    type State = DatabaseName;

    fn make_request(self) -> Result<(T::Request, Self::State), Error> {
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

        let options = match self.limit {
            None => options,
            Some(value) => options.with_limit(value),
        };

        let options = match self.descending {
            None => options,
            Some(value) => options.with_descending_query(value),
        };

        let view_path = try!(self.view_path.into_view_path());
        let db_name = view_path.database_name().clone();
        let request = try!(self.transport.get(view_path.iter(), options));
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

    use super::*;
    use {DatabaseName, Error, ViewPath};
    use serde_json;
    use transport::{Action, RequestOptions, StatusCode, Transport};
    use transport::testing::{MockResponse, MockTransport};
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
            let action = ExecuteView::<_, &'static str, String, i32>::new(&transport,
                                                                          "/foo/_design/bar/_view/qux");
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
            let action =
                ExecuteView::<_, &'static str, String, i32>::new(&transport,
                                                                 "/foo/_design/bar/_view/qux")
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
            let action =
                ExecuteView::<_, &'static str, String, i32>::new(&transport,
                                                                 "/foo/_design/bar/_view/qux")
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
            let action =
                ExecuteView::<_, &'static str, String, i32>::new(&transport,
                                                                 "/foo/_design/bar/_view/qux")
                    .with_end_key_inclusive(&end_key);
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn make_request_with_limit() {
        let transport = MockTransport::new();

        let expected = ({
            let options = RequestOptions::new()
                              .with_limit(42)
                              .with_accept_json();
            transport.get(vec!["foo", "_design", "bar", "_view", "qux"], options).unwrap()
        },
                        DatabaseName::from("foo"));

        let got = {
            let action =
                ExecuteView::<_, &'static str, String, i32>::new(&transport,
                                                                 "/foo/_design/bar/_view/qux")
                    .with_limit(42);
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
            let action =
                ExecuteView::<_, &'static str, String, i32>::new(&transport,
                                                                 "/foo/_design/bar/_view/qux")
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
            let action =
                ExecuteView::<_, &'static str, String, i32>::new(&transport,
                                                                 "/foo/_design/bar/_view/qux")
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

        let got = ExecuteView::<MockTransport,
                                ViewPath,
                                _,
                                _>::take_response(response, DatabaseName::from("foo"))
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
        let got = ExecuteView::<MockTransport, ViewPath, _, _>::take_response(response, db_name)
                      .unwrap();
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
        match ExecuteView::<MockTransport, ViewPath, String, i32>::take_response(response,
                                                                                 db_name) {
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
        match ExecuteView::<MockTransport, ViewPath, String, i32>::take_response(response,
                                                                                 db_name) {
            Err(Error::Unauthorized(ref error_response)) if error == error_response.error() &&
                                                            reason == error_response.reason() => (),
            x @ _ => unexpected_result!(x),
        }
    }
}
