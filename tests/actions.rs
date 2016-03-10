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
    client.create_database("baseball", Default::default()).unwrap();
    // FIXME: Verify that the database was created.
}

#[test]
fn create_database_nok_database_exists() {
    let (_server, client) = make_server_and_client();
    client.create_database("baseball", Default::default()).unwrap();
    match client.create_database("baseball", Default::default()) {
        Err(chill::Error::DatabaseExists(..)) => (),
        x @ _ => {
            panic!("Unexpected result: {:?}", x);
        }
    }
}

#[test]
fn create_document_ok_default_options() {

    let (_server, client) = make_server_and_client();
    client.create_database("baseball", Default::default()).unwrap();
    let db = client.select_database("baseball");

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Babe Ruth")
                         .insert("nickname", "The Bambino")
                         .unwrap();

    let (doc_id, _rev) = db.create_document(&up_content, Default::default()).unwrap();

    let doc = db.read_document(doc_id, Default::default()).unwrap();
    let down_content = doc.get_content().unwrap();
    assert_eq!(up_content, down_content);
}

#[test]
fn create_document_ok_with_document_id() {

    let (_server, client) = make_server_and_client();
    client.create_database("baseball", Default::default()).unwrap();
    let db = client.select_database("baseball");

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Babe Ruth")
                         .insert("nickname", "The Bambino")
                         .unwrap();

    let (doc_id, _rev) = db.create_document(&up_content,
                                            chill::CreateDocumentOptions::new()
                                                .with_document_id("babe_ruth"))
                           .unwrap();

    assert_eq!(chill::DocumentId::from("babe_ruth"), doc_id);

    let doc = db.read_document(doc_id, Default::default()).unwrap();
    let down_content = doc.get_content().unwrap();
    assert_eq!(up_content, down_content);
}

#[test]
fn create_document_nok_document_conflict() {

    let (_server, client) = make_server_and_client();
    client.create_database("baseball", Default::default()).unwrap();
    let db = client.select_database("baseball");

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Babe Ruth")
                         .insert("nickname", "The Bambino")
                         .unwrap();

    let (doc_id, _rev) = db.create_document(&up_content, Default::default()).unwrap();

    match db.create_document(&up_content,
                             chill::CreateDocumentOptions::new().with_document_id(doc_id)) {
        Err(chill::Error::DocumentConflict(..)) => (),
        x @ _ => unexpected_result!(x),
    }
}

// FIXME: Implement this test.
// #[test]
// fn create_document_nok_unauthorized() {
//     unimplemented!();
// }

#[test]
fn read_document_ok_default_options() {

    let (_server, client) = make_server_and_client();
    client.create_database("baseball", Default::default()).unwrap();
    let db = client.select_database("baseball");

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Babe Ruth")
                         .insert("nickname", "The Bambino")
                         .unwrap();

    let (doc_id, _rev) = db.create_document(&up_content, Default::default()).unwrap();

    let doc = db.read_document(doc_id, Default::default()).unwrap();
    let down_content = doc.get_content().unwrap();
    assert_eq!(up_content, down_content);
}

#[test]
fn read_document_nok_not_found() {

    let (_server, client) = make_server_and_client();
    client.create_database("baseball", Default::default()).unwrap();
    let db = client.select_database("baseball");

    match db.read_document("babe_ruth", Default::default()) {
        Err(chill::Error::NotFound(..)) => (),
        x @ _ => unexpected_result!(x),
    }
}

// FIXME: Implement this test.
// #[test]
// fn read_document_nok_unauthorized() {
//     unimplemented!();
// }

#[test]
fn update_document_ok() {

    let (_server, client) = make_server_and_client();
    client.create_database("baseball", Default::default()).unwrap();
    let db = client.select_database("baseball");

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Babe Ruth")
                         .insert("nickname", "The Bambino")
                         .unwrap();

    let (doc_id, _rev) = db.create_document(&up_content, Default::default()).unwrap();

    let mut doc = db.read_document(doc_id.clone(), Default::default()).unwrap();

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

    let updated_rev = db.update_document(&doc, Default::default()).unwrap();

    let doc = db.read_document(doc_id, Default::default()).unwrap();
    let down_content: serde_json::Value = doc.get_content().unwrap();
    assert_eq!(up_content, down_content);
    assert_eq!(&updated_rev, doc.revision());
}

#[test]
fn delete_document_ok() {

    let (_server, client) = make_server_and_client();
    client.create_database("baseball", Default::default()).unwrap();
    let db = client.select_database("baseball");

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Babe Ruth")
                         .insert("nickname", "The Bambino")
                         .unwrap();

    let (doc_id, rev1) = db.create_document(&up_content, Default::default()).unwrap();

    let _rev2 = db.delete_document(doc_id.clone(), &rev1, Default::default()).unwrap();

    match db.read_document(doc_id.clone(), Default::default()) {
        Err(chill::Error::NotFound(..)) => (),
        x @ _ => {
            panic!("Unexpected result: {:?}", x);
        }
    }

    // FIXME: Check that _rev2 is valid. We need to be able to get a document at
    // a specific revision to check this.
}
