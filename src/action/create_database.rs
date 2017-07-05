use Error;
use futures::Future;
use transport::{Method, Request, Response, StatusCode, Transport};

#[derive(Debug)]
pub struct CreateDatabase<'a, T: Transport + 'a> {
    transport: &'a T,
    db_path: String,
}

impl<'a, T: Transport> CreateDatabase<'a, T> {
    #[doc(hidden)]
    pub fn new(transport: &'a T, db_path: &str) -> Self {
        CreateDatabase {
            transport: transport,
            db_path: String::from(db_path),
        }
    }

    pub fn send(self) -> Box<Future<Item = (), Error = Error>> {
        Box::new(
            self.transport
                .request(Method::Put, &format!("/{}", self.db_path)) // # FIXME: Better type-safety for db_path
                .send()
                .and_then(|response| {
                    match response.status_code() {
                        StatusCode::Created => Ok(()),
                        StatusCode::PreconditionFailed => Err(Error::DatabaseExists),
                        _ => Err(response.into_error()),
                    }
                }),
        )
    }
}
