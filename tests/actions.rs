extern crate chill;
extern crate serde_json;

fn make_server_and_client() -> (chill::testing::FakeServer, chill::Client) {
    let server = chill::testing::FakeServer::new().unwrap();
    let client = chill::Client::new(server.uri()).unwrap();
    (server, client)
}

#[test]
fn create_database_ok() {
    let (_server, client) = make_server_and_client();
    client.create_database("foo").run().unwrap();
    // FIXME: Check that the returned database is valid.
}

#[test]
fn create_document_ok() {

    let (_server, client) = make_server_and_client();
    let db = client.create_database("foo").run().unwrap();

    let content = serde_json::builder::ObjectBuilder::new().unwrap();
    db.create_document(&content).run().unwrap();

    // FIXME: Check that the returned document is valid.
}
