extern crate chill;

fn make_server_and_client() -> (chill::testing::FakeServer, chill::Client) {
    let server = chill::testing::FakeServer::new().unwrap();
    let client = chill::Client::new(server.uri()).unwrap();
    (server, client)
}

#[test]
fn create_database_ok() {
    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").send().wait().unwrap();
}
