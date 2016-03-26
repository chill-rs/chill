extern crate chill;
extern crate serde_json;

macro_rules! unexpected_result {
    ($result:expr) => {
        match $result {
            Err(e) => panic!("Got unexpected error result {:?}", e),
            Ok(x) => panic!("Got unexpected OK result {:?}", x),
        }
    }
}

fn make_server_and_client() -> (chill::testing::FakeServer, chill::Client) {
    let server = chill::testing::FakeServer::new().unwrap();
    let client = chill::Client::new(server.uri()).unwrap();
    (server, client)
}

#[test]
fn create_database_ok() {
    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").unwrap().run().unwrap();
}

#[test]
fn create_database_nok_database_exists() {
    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").unwrap().run().unwrap();
    match client.create_database("/baseball").unwrap().run() {
        Err(chill::Error::DatabaseExists(..)) => (),
        x @ _ => {
            panic!("Unexpected result: {:?}", x);
        }
    }
}

#[test]
fn create_document_ok_default() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").unwrap().run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Babe Ruth")
                         .insert("nickname", "The Bambino")
                         .unwrap();

    let (doc_id, _rev) = client.create_document("/baseball", &up_content).unwrap().run().unwrap();

    let doc = client.read_document(("/baseball", &doc_id))
                    .unwrap()
                    .run()
                    .unwrap();
    let down_content = doc.get_content().unwrap();
    assert_eq!(up_content, down_content);
}

#[test]
fn create_document_ok_with_document_id() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").unwrap().run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Babe Ruth")
                         .insert("nickname", "The Bambino")
                         .unwrap();

    let (doc_id, _rev) = client.create_document("/baseball", &up_content)
                               .unwrap()
                               .with_document_id("babe_ruth")
                               .run()
                               .unwrap();
    assert_eq!(chill::DocumentId::from("babe_ruth"), doc_id);

    let doc = client.read_document(("/baseball", &doc_id))
                    .unwrap()
                    .run()
                    .unwrap();
    let down_content = doc.get_content().unwrap();
    assert_eq!(up_content, down_content);
}

#[test]
fn create_document_nok_document_conflict() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").unwrap().run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Babe Ruth")
                         .insert("nickname", "The Bambino")
                         .unwrap();

    let (doc_id, _rev) = client.create_document("/baseball", &up_content).unwrap().run().unwrap();

    match client.create_document("/baseball", &up_content)
                .unwrap()
                .with_document_id(&doc_id)
                .run() {
        Err(chill::Error::DocumentConflict(..)) => (),
        x @ _ => unexpected_result!(x),
    }
}

#[test]
fn read_document_ok_default() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").unwrap().run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Babe Ruth")
                         .insert("nickname", "The Bambino")
                         .unwrap();

    let (doc_id, _rev) = client.create_document("/baseball", &up_content).unwrap().run().unwrap();

    let doc = client.read_document(("/baseball", &doc_id))
                    .unwrap()
                    .run()
                    .unwrap();
    let down_content = doc.get_content().unwrap();
    assert_eq!(up_content, down_content);
}

#[test]
fn read_document_ok_with_revision() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").unwrap().run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Babe Ruth")
                         .insert("nickname", "The Bambino")
                         .unwrap();

    let (doc_id, rev) = client.create_document("/baseball", &up_content).unwrap().run().unwrap();

    let doc = client.read_document(("/baseball", &doc_id))
                    .unwrap()
                    .with_revision(&rev)
                    .run()
                    .unwrap();
    let down_content = doc.get_content().unwrap();
    assert_eq!(up_content, down_content);
}

#[test]
fn read_document_nok_not_found() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").unwrap().run().unwrap();

    match client.read_document("/baseball/babe_ruth").unwrap().run() {
        Err(chill::Error::NotFound(..)) => (),
        x @ _ => unexpected_result!(x),
    }
}

#[test]
fn update_document_ok() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").unwrap().run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Babe Ruth")
                         .insert("nickname", "The Bambino")
                         .unwrap();

    let (doc_id, _rev) = client.create_document("/baseball", &up_content).unwrap().run().unwrap();

    let mut doc = client.read_document(("/baseball", &doc_id))
                        .unwrap()
                        .run()
                        .unwrap();

    let up_content = match doc.get_content::<serde_json::Value>().unwrap() {
        serde_json::Value::Object(mut fields) => {
            fields.insert("birthday".to_string(),
                          serde_json::Value::String("1895-02-06".to_string()));
            serde_json::Value::Object(fields)
        }
        _ => {
            panic!("Invalid JSON type");
        }
    };

    doc.set_content(&up_content).unwrap();

    let updated_rev = client.update_document(&doc).unwrap().run().unwrap();

    let doc = client.read_document(("/baseball", doc.id()))
                    .unwrap()
                    .run()
                    .unwrap();
    let down_content: serde_json::Value = doc.get_content().unwrap();
    assert_eq!(up_content, down_content);
    assert_eq!(&updated_rev, doc.revision());
}

#[test]
fn delete_document_ok() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").unwrap().run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Babe Ruth")
                         .insert("nickname", "The Bambino")
                         .unwrap();

    let (doc_id, rev1) = client.create_document("/baseball", &up_content).unwrap().run().unwrap();

    let rev2 = client.delete_document(("/baseball", &doc_id), &rev1)
                     .unwrap()
                     .run()
                     .unwrap();

    match client.read_document(("/baseball", &doc_id)).unwrap().run() {
        Err(chill::Error::NotFound(..)) => (),
        x @ _ => {
            panic!("Unexpected result: {:?}", x);
        }
    }

    let doc = client.read_document(("/baseball", &doc_id))
                    .unwrap()
                    .with_revision(&rev2)
                    .run()
                    .unwrap();
    assert!(doc.is_deleted());
}
