use DatabaseName;
use AllDocumentsViewValue;
use ViewResponse;
use DocumentId;
use IntoDatabasePath;
use transport::{Action, Response, Transport};
use transport::production::HyperTransport;
use Error;

use super::ExecuteView;

pub struct ReadAllDocuments<'a, T>
    where T: Transport + 'a
{
    execute_view: ExecuteView<'a, T, DocumentId, AllDocumentsViewValue>
}

impl<'a, T> ReadAllDocuments<'a, T>
    where T: Transport
{
    #[doc(hidden)]
    pub fn new<P: IntoDatabasePath<'a>>(transport: &'a T, database_path: P) -> Result<Self, Error> {
        Ok(ReadAllDocuments {
            execute_view: try!(ExecuteView::with_database_view(transport, (database_path, "_all_docs")))
        })
    }

    /// Modified the action to include only records with a key greater than or
    /// equal to a given key.
    ///
    /// The `with_start_key` method abstracts CouchDB's `startkey` query
    /// parameter. By default, the CouchDB server includes all records.
    ///
    pub fn with_start_key(mut self, start_key: &'a DocumentId) -> Self {
        self.execute_view = self.execute_view.with_start_key(start_key);
        self
    }

    /// Modified the action to include only records with a key less than or
    /// equal to a given key.
    ///
    /// The `with_end_key_inclusive` method abstracts CouchDB's `endkey` query
    /// parameter. By default, the CouchDB server includes all records.
    ///
    pub fn with_end_key_inclusive(mut self, end_key: &'a DocumentId) -> Self {
        self.execute_view = self.execute_view.with_end_key_inclusive(end_key);
        self
    }

    /// Modified the action to include only records with a key less than a given
    /// key.
    ///
    /// The `with_end_key_exclusive` method abstracts CouchDB's `endkey` and
    /// `inclusive_end` query parameters. By default, the CouchDB server
    /// includes all records.
    ///
    pub fn with_end_key_exclusive(mut self, end_key: &'a DocumentId) -> Self {
        self.execute_view = self.execute_view.with_end_key_exclusive(end_key);
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
        self.execute_view = self.execute_view.with_descending(descending);
        self
    }
}

impl<'a> ReadAllDocuments<'a, HyperTransport>
{
    pub fn run(self) -> Result<ViewResponse<DocumentId, AllDocumentsViewValue>, Error> {
        self.execute_view.run()
    }
}

impl<'a, T: Transport + 'a> Action<T> for ReadAllDocuments<'a, T> {
    type Output = ViewResponse<DocumentId, AllDocumentsViewValue>;
    type State = DatabaseName;

    fn make_request(&mut self) -> Result<(T::Request, Self::State), Error> {
        self.execute_view.make_request()
    }

    fn take_response<R: Response>(response: R,
                                  db_name: Self::State)
                                  -> Result<Self::Output, Error> {
        ExecuteView::<T, DocumentId, AllDocumentsViewValue>::take_response(response, db_name)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use DatabaseName;
    use transport::{Action, RequestOptions, Transport};
    use transport::testing::{MockTransport};

    #[test]
    fn make_request_default() {
        let transport = MockTransport::new();

        let expected = ({
            let options = RequestOptions::new().with_accept_json();
            transport.get(vec!["foo", "_all_docs"], options).unwrap()
        },
                        DatabaseName::from("foo"));

        let got = {
            let mut action = ReadAllDocuments::new(&transport,
                                                                "/foo")
                                 .unwrap();
            action.make_request().unwrap()
        };

        assert_eq!(expected, got);
    }

}