//! Defines an action for executing a view.

use {DatabaseName, Error, IntoViewPath, ViewResponse};
use transport::{Action, RequestOptions, Response, StatusCode, Transport};
use transport::production::HyperTransport;
use view::ViewResponseJsonable;
use serde;

enum Inclusivity {
    Exclusive,
    Inclusive,
}

enum GroupLevel {
    Exact(bool),
    Number(u32),
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
/// let view_response = client.execute_view(
///                               "/baseball/_design/stat/_view/home_run")
///                           .with_descending(true)
///                           .with_end_key_inclusive(&700)
///                           .run()
///                           .unwrap();
///
/// let expected = vec![
///     "Hank Aaron - 755",
///     "Babe Ruth - 714",
/// ];
///
/// let got = view_response.rows()
///                        .iter()
///                        .map(|x| format!("{} - {}",
///                                        x.value::<String>().unwrap(),
///                                        x.key::<i32>().unwrap().unwrap()))
///                        .collect::<Vec<_>>();
///
/// assert_eq!(expected, got);
/// ```
///
pub struct ExecuteView<'a, T, P, StartKey, EndKey>
    where EndKey: serde::Serialize,
          P: IntoViewPath,
          StartKey: serde::Serialize,
          T: Transport + 'a
{
    transport: &'a T,
    view_path: P,
    reduce: Option<bool>,
    start_key: Option<StartKey>,
    end_key: Option<(EndKey, Inclusivity)>,
    limit: Option<u64>,
    descending: Option<bool>,
    group_level: Option<GroupLevel>,
}

impl<'a, P, T> ExecuteView<'a, T, P, (), ()>
    where P: IntoViewPath,
          T: Transport + 'a
{
    #[doc(hidden)]
    pub fn new(transport: &'a T, view_path: P) -> Self {
        ExecuteView {
            transport: transport,
            view_path: view_path,
            reduce: None,
            start_key: None,
            end_key: None,
            limit: None,
            descending: None,
            group_level: None,
        }
    }
}

impl<'a, EndKey, P, StartKey, T> ExecuteView<'a, T, P, StartKey, EndKey>
    where EndKey: serde::Serialize,
          P: IntoViewPath,
          StartKey: serde::Serialize,
          T: Transport + 'a
{
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

    pub fn with_exact_groups(mut self, yes_or_no: bool) -> Self {
        self.group_level = Some(GroupLevel::Exact(yes_or_no));
        self
    }

    pub fn with_group_level(mut self, group_level: u32) -> Self {
        self.group_level = Some(GroupLevel::Number(group_level));
        self
    }
}

impl<'a, EndKey, P, T> ExecuteView<'a, T, P, (), EndKey>
    where EndKey: serde::Serialize,
          P: IntoViewPath,
          T: Transport + 'a
{
    /// Modifies the action to include only records with a key greater than or
    /// equal to a given key.
    ///
    /// The `with_start_key` method abstracts CouchDB's `startkey` query
    /// parameter. By default, the CouchDB server includes all records.
    ///
    pub fn with_start_key<StartKey>(self,
                                    start_key: StartKey)
                                    -> ExecuteView<'a, T, P, StartKey, EndKey>
        where StartKey: serde::Serialize
    {
        ExecuteView {
            transport: self.transport,
            view_path: self.view_path,
            reduce: self.reduce,
            start_key: Some(start_key),
            end_key: self.end_key,
            limit: self.limit,
            descending: self.descending,
            group_level: self.group_level,
        }
    }
}

impl<'a, P, StartKey, T> ExecuteView<'a, T, P, StartKey, ()>
    where P: IntoViewPath,
          StartKey: serde::Serialize,
          T: Transport + 'a
{
    /// Modifies the action to include only records with a key less than or
    /// equal to a given key.
    ///
    /// The `with_end_key_inclusive` method abstracts CouchDB's `endkey` query
    /// parameter. By default, the CouchDB server includes all records.
    ///
    pub fn with_end_key_inclusive<EndKey>(self,
                                          end_key: EndKey)
                                          -> ExecuteView<'a, T, P, StartKey, EndKey>
        where EndKey: serde::Serialize
    {
        ExecuteView {
            transport: self.transport,
            view_path: self.view_path,
            reduce: self.reduce,
            start_key: self.start_key,
            end_key: Some((end_key, Inclusivity::Inclusive)),
            limit: self.limit,
            descending: self.descending,
            group_level: self.group_level,
        }
    }

    /// Modifies the action to include only records with a key less than a given
    /// key.
    ///
    /// The `with_end_key_exclusive` method abstracts CouchDB's `endkey` and
    /// `inclusive_end` query parameters. By default, the CouchDB server
    /// includes all records.
    ///
    pub fn with_end_key_exclusive<EndKey>(self,
                                          end_key: EndKey)
                                          -> ExecuteView<'a, T, P, StartKey, EndKey>
        where EndKey: serde::Serialize
    {
        ExecuteView {
            transport: self.transport,
            view_path: self.view_path,
            reduce: self.reduce,
            start_key: self.start_key,
            end_key: Some((end_key, Inclusivity::Exclusive)),
            limit: self.limit,
            descending: self.descending,
            group_level: self.group_level,
        }
    }
}

impl<'a, EndKey, P, StartKey> ExecuteView<'a, HyperTransport, P, StartKey, EndKey>
    where EndKey: serde::Serialize,
          P: IntoViewPath,
          StartKey: serde::Serialize
{
    pub fn run(self) -> Result<ViewResponse, Error> {
        self.transport.exec_sync(self)
    }
}

impl<'a, P, T, StartKey, EndKey> Action<T> for ExecuteView<'a, T, P, StartKey, EndKey>
    where EndKey: serde::Serialize,
          P: IntoViewPath,
          StartKey: serde::Serialize,
          T: Transport + 'a
{
    type Output = ViewResponse;
    type State = DatabaseName;

    fn make_request(self) -> Result<(T::Request, Self::State), Error> {
        let options = RequestOptions::new().with_accept_json();

        let options = match self.reduce {
            None => options,
            Some(value) => options.with_reduce_query(value),
        };

        let options = match self.start_key {
            None => options,
            Some(ref value) => try!(options.with_start_key(value)),
        };

        let options = match self.end_key {
            None => options,
            Some((ref key_value, Inclusivity::Inclusive)) => try!(options.with_end_key(key_value)),
            Some((ref key_value, Inclusivity::Exclusive)) => {
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

        let options = match self.group_level {
            None => options,
            Some(GroupLevel::Exact(value)) => options.with_group(value),
            Some(GroupLevel::Number(value)) => options.with_group_level(value),
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
                let body: ViewResponseJsonable = try!(response.decode_json_body());
                Ok(ViewResponse::new_from_decoded(db_name, body))
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
            let action = ExecuteView::new(&transport, "/foo/_design/bar/_view/qux");
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
            let action = ExecuteView::new(&transport, "/foo/_design/bar/_view/qux")
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
            let action = ExecuteView::new(&transport, "/foo/_design/bar/_view/qux")
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
            let action = ExecuteView::new(&transport, "/foo/_design/bar/_view/qux")
                             .with_end_key_inclusive(&end_key);
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn make_request_with_exact_groups() {

        let transport = MockTransport::new();

        let expected = ({
            let options = RequestOptions::new()
                              .with_group(true)
                              .with_accept_json();
            transport.get(vec!["foo", "_design", "bar", "_view", "qux"], options).unwrap()
        },
                        DatabaseName::from("foo"));

        let got = {
            let action = ExecuteView::new(&transport, "/foo/_design/bar/_view/qux")
                             .with_exact_groups(true);
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

    #[test]
    fn make_request_with_group_level() {

        let transport = MockTransport::new();

        let expected = ({
            let options = RequestOptions::new()
                              .with_group_level(42)
                              .with_accept_json();
            transport.get(vec!["foo", "_design", "bar", "_view", "qux"], options).unwrap()
        },
                        DatabaseName::from("foo"));

        let got = {
            let action = ExecuteView::new(&transport, "/foo/_design/bar/_view/qux")
                             .with_group_level(42);
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
            let action = ExecuteView::new(&transport, "/foo/_design/bar/_view/qux").with_limit(42);
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
            let action = ExecuteView::new(&transport, "/foo/_design/bar/_view/qux")
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
            let action = ExecuteView::new(&transport, "/foo/_design/bar/_view/qux")
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
                                (),
                                ()>::take_response(response, DatabaseName::from("foo"))
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

        let expected = ViewResponseBuilder::new_unreduced("baseball", 20, 10)
                           .with_row("babe_ruth", "Babe Ruth", 714)
                           .with_row("hank_aaron", "Hank Aaron", 755)
                           .unwrap();

        let db_name = DatabaseName::from("baseball");
        let got = ExecuteView::<MockTransport, ViewPath, (), ()>::take_response(response, db_name)
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
