use {Error, futures};
use futures::Future;
use transport::{Method, Request, Response, StatusCode, Transport};

/// `CreateDatabase` creates a database.
///
/// The return type implements `Future<Item = (), Error = Error>`.
///
#[derive(Debug)]
pub struct CreateDatabase<'a, T: Transport + 'a> {
    transport: &'a T,
    db_path: String,
}

/// `CreateDatabaseFuture` is a future for a `CreateDatabase` action.
///
/// The `CreateDatabaseFuture` type implements `Future<Item = (), Error =
/// Error>`.
///
pub type CreateDatabaseFuture<T> =
    futures::future::AndThen<
        <<T as Transport>::Request as Request>::Future,
        Result<(), Error>,
        fn(<<T as Transport>::Request as Request>::Response) -> Result<(), Error>,
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

        fn f1<T: Transport>(response: <<T as Transport>::Request as Request>::Response) -> Result<(), Error> {
            match response.status_code() {
                StatusCode::Created => Ok(()),
                StatusCode::PreconditionFailed => Err(Error::DatabaseExists),
                _ => Err(response.into_error()),
            }
        }

        self.transport
                .request(Method::Put, &format!("/{}", self.db_path)) // # FIXME: Better type-safety for db_path
                .send()
                .and_then(f1::<T>)
    }
}
