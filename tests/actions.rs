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
    client.create_database("foo", Default::default()).unwrap();
}

#[test]
fn create_document_ok() {

    let (_server, client) = make_server_and_client();
    client.create_database("foo", Default::default()).unwrap();
    let db = client.select_database("foo");

    let content = serde_json::builder::ObjectBuilder::new().unwrap();
    let (_doc_id, _rev) = db.create_document(&content, Default::default()).unwrap();

    // FIXME: Verify that the returned document id and revision are correct.
}
