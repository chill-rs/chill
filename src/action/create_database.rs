use {ActionError, futures};
use futures::Future;
use transport::{Method, Request, Response, StatusCode, Transport};

/// `CreateDatabase` creates a database.
///
/// The return type implements `Future<Item = (), Error = ActionError>`.
///
#[derive(Debug)]
pub struct CreateDatabase<'a, T: Transport + 'a> {
    transport: &'a T,
    db_path: String,
}

/// `CreateDatabaseFuture` is a future for a `CreateDatabase` action.
///
/// The `CreateDatabaseFuture` type implements `Future<Item = (), Error =
/// ActionError>`.
///
pub type CreateDatabaseFuture<T> =
    futures::future::AndThen<
        <<T as Transport>::Request as Request>::Future,
        Result<(), ActionError>,
        fn(<<T as Transport>::Request as Request>::Response) -> Result<(), ActionError>,
    >;

impl<'a, T: Transport> CreateDatabase<'a, T> {
    #[doc(hidden)]
    pub fn new(transport: &'a T, db_path: &str) -> Self {
        CreateDatabase {
            transport: transport,
            db_path: String::from(db_path),
        }
    }

    pub fn send(self) -> CreateDatabaseFuture<T> {

        fn f1<T: Transport>(response: <<T as Transport>::Request as Request>::Response) -> Result<(), ActionError> {
            match response.status_code() {
                StatusCode::Created => Ok(()),
                StatusCode::PreconditionFailed => Err(ActionError::DatabaseExists),
                x => Err(ActionError::Other(
                    format!("CouchDB server responded with {}", x),
                )),
            }
        }

        self.transport
                .request(Method::Put, &format!("/{}", self.db_path)) // # FIXME: Better type-safety for db_path
                .send()
                .and_then(f1::<T>)
    }
}
