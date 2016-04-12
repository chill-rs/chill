extern crate base64;
extern crate chill;
#[macro_use]
extern crate mime;
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
fn read_document_ok_with_attachment_stubs() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").unwrap().run().unwrap();

    // TODO: Create the attachment using strong types.

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Babe Ruth")
                         .insert("nickname", "The Bambino")
                         .insert_object("_attachments", |x| {
                             x.insert_object("photo.png", |x| {
                                 x.insert("content_type", "image/png")
                                  .insert("data",
                                          base64::encode("Pretend this is a PNG file.").unwrap())
                             })
                         })
                         .unwrap();

    let (doc_id, _rev) = client.create_document("/baseball", &up_content).unwrap().run().unwrap();

    let doc = client.read_document(("/baseball", &doc_id))
                    .unwrap()
                    .run()
                    .unwrap();

    let expected_attachments = {
        let mut m = std::collections::HashMap::<chill::AttachmentPath, (mime::Mime, u64)>::new();
        let path = chill::AttachmentPath::from((doc.path().clone(),
                                                chill::AttachmentName::from("photo.png")));
        m.insert(path,
                 (mime!(Image / Png),
                  "Pretend this is a PNG file.".len() as u64));
        m
    };

    let got_attachments = doc.attachments()
                             .map(|(name, attachment)| {
                                 let path = chill::AttachmentPath::from(name);
                                 let content_type = attachment.content_type().clone();
                                 let content_length = attachment.content_length();
                                 (path, (content_type, content_length))
                             })
                             .collect();

    assert_eq!(expected_attachments, got_attachments);
}

#[test]
fn read_document_ok_with_attachment_content() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").unwrap().run().unwrap();

    // TODO: Create the attachment using strong types.

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Babe Ruth")
                         .insert("nickname", "The Bambino")
                         .insert_object("_attachments", |x| {
                             x.insert_object("photo.png", |x| {
                                 x.insert("content_type", "image/png")
                                  .insert("data",
                                          base64::encode("Pretend this is a PNG file.").unwrap())
                             })
                         })
                         .unwrap();

    let (doc_id, _rev) = client.create_document("/baseball", &up_content).unwrap().run().unwrap();

    let doc = client.read_document(("/baseball", &doc_id))
                    .unwrap()
                    .with_attachment_content(chill::action::read_document::AttachmentContent::All)
                    .run()
                    .unwrap();

    let expected_attachments = {
        let mut m =
            std::collections::HashMap::<chill::AttachmentPath, (mime::Mime, Vec<u8>)>::new();
        let path = chill::AttachmentPath::from((doc.path().clone(),
                                                chill::AttachmentName::from("photo.png")));
        m.insert(path,
                 (mime!(Image / Png), Vec::from("Pretend this is a PNG file.")));
        m
    };

    let got_attachments = doc.attachments()
                             .map(|(name, attachment)| {
                                 let path = chill::AttachmentPath::from(name);
                                 let content_type = attachment.content_type().clone();
                                 let content = attachment.content().unwrap().clone();
                                 (path, (content_type, content))
                             })
                             .collect();

    assert_eq!(expected_attachments, got_attachments);
}

#[test]
fn update_document_ok_default() {

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
fn update_document_ok_create_attachment() {

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

    doc.insert_attachment("photo.png",
                          mime!(Image / Png),
                          Vec::from("Pretend this is a PNG file."));

    client.update_document(&doc).unwrap().run().unwrap();

    let doc = client.read_document(("/baseball", &doc_id))
                    .unwrap()
                    .with_attachment_content(chill::action::read_document::AttachmentContent::All)
                    .run()
                    .unwrap();

    let expected_attachments = {
        let mut m =
            std::collections::HashMap::<chill::AttachmentPath, (mime::Mime, Vec<u8>)>::new();
        let path = chill::AttachmentPath::from((doc.path().clone(),
                                                chill::AttachmentName::from("photo.png")));
        m.insert(path,
                 (mime!(Image / Png), Vec::from("Pretend this is a PNG file.")));
        m
    };

    let got_attachments = doc.attachments()
                             .map(|(name, attachment)| {
                                 let path = chill::AttachmentPath::from(name);
                                 let content_type = attachment.content_type().clone();
                                 let content = attachment.content().unwrap().clone();
                                 (path, (content_type, content))
                             })
                             .collect();

    assert_eq!(expected_attachments, got_attachments);
}

#[test]
fn update_document_ok_update_attachment() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").unwrap().run().unwrap();

    // TODO: Create the attachment using strong types.

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Babe Ruth")
                         .insert("nickname", "The Bambino")
                         .insert_object("_attachments", |x| {
                             x.insert_object("photo.png", |x| {
                                 x.insert("content_type", "image/png")
                                  .insert("data",
                                          base64::encode("Pretend this is a PNG file.").unwrap())
                             })
                         })
                         .unwrap();

    let (doc_id, _rev) = client.create_document("/baseball", &up_content).unwrap().run().unwrap();

    let mut doc = client.read_document(("/baseball", &doc_id))
                        .unwrap()
                        .run()
                        .unwrap();

    doc.insert_attachment("photo.png",
                          mime!(Image / Png),
                          Vec::from("Pretend we updated the photo."));

    client.update_document(&doc).unwrap().run().unwrap();

    let doc = client.read_document(("/baseball", &doc_id))
                    .unwrap()
                    .with_attachment_content(chill::action::read_document::AttachmentContent::All)
                    .run()
                    .unwrap();

    let expected_attachments = {
        let mut m =
            std::collections::HashMap::<chill::AttachmentPath, (mime::Mime, Vec<u8>)>::new();
        let path = chill::AttachmentPath::from((doc.path().clone(),
                                                chill::AttachmentName::from("photo.png")));
        m.insert(path,
                 (mime!(Image / Png),
                  Vec::from("Pretend we updated the photo.")));
        m
    };

    let got_attachments = doc.attachments()
                             .map(|(name, attachment)| {
                                 let path = chill::AttachmentPath::from(name);
                                 let content_type = attachment.content_type().clone();
                                 let content = attachment.content().unwrap().clone();
                                 (path, (content_type, content))
                             })
                             .collect();

    assert_eq!(expected_attachments, got_attachments);
}

#[test]
fn update_document_ok_delete_attachment() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").unwrap().run().unwrap();

    // TODO: Create the attachment using strong types.

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Babe Ruth")
                         .insert("nickname", "The Bambino")
                         .insert_object("_attachments", |x| {
                             x.insert_object("photo.png", |x| {
                                 x.insert("content_type", "image/png")
                                  .insert("data",
                                          base64::encode("Pretend this is a PNG file.").unwrap())
                             })
                         })
                         .unwrap();

    let (doc_id, _rev) = client.create_document("/baseball", &up_content).unwrap().run().unwrap();

    let mut doc = client.read_document(("/baseball", &doc_id))
                        .unwrap()
                        .run()
                        .unwrap();

    doc.remove_attachment("photo.png");

    client.update_document(&doc).unwrap().run().unwrap();

    let doc = client.read_document(("/baseball", &doc_id))
                    .unwrap()
                    .with_attachment_content(chill::action::read_document::AttachmentContent::All)
                    .run()
                    .unwrap();

    let expected_attachments = std::collections::HashMap::<chill::AttachmentPath,
                                                           (mime::Mime, Vec<u8>)>::new();

    let got_attachments = doc.attachments()
                             .map(|(name, attachment)| {
                                 let path = chill::AttachmentPath::from(name);
                                 let content_type = attachment.content_type().clone();
                                 let content = attachment.content().unwrap().clone();
                                 (path, (content_type, content))
                             })
                             .collect();

    assert_eq!(expected_attachments, got_attachments);
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

#[test]
fn execute_view_ok_unreduced_default() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").unwrap().run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Babe Ruth")
                         .insert("home_runs", 714)
                         .unwrap();

    let (babe_id, _) = client.create_document("/baseball", &up_content).unwrap().run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Hank Aaron")
                         .insert("home_runs", 755)
                         .unwrap();

    let (hank_id, _) = client.create_document("/baseball", &up_content).unwrap().run().unwrap();

    // TODO: Make use of a Design type when available.

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert_object("views", |x| {
                             x.insert_object("home_runs", |x| {
                                 x.insert("map",
                                          "function(doc) { emit(doc.home_runs, doc.home_runs) }")
                             })
                         })
                         .unwrap();
    client.create_document("/baseball", &up_content)
          .unwrap()
          .with_document_id("_design/stats")
          .run()
          .unwrap();

    let expected = chill::testing::ViewResponseBuilder::new_unreduced(2, 0, "baseball")
                       .with_row(714, 714, babe_id)
                       .with_row(755, 755, hank_id)
                       .unwrap();

    let got = client.execute_view::<i32, i32, _>("/baseball/_design/stats/_view/home_runs")
                    .unwrap()
                    .run()
                    .unwrap();

    assert_eq!(expected, got);
}

#[test]
fn execute_view_ok_unreduced_with_descending() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").unwrap().run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Babe Ruth")
                         .insert("home_runs", 714)
                         .unwrap();

    let (babe_id, _) = client.create_document("/baseball", &up_content).unwrap().run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Hank Aaron")
                         .insert("home_runs", 755)
                         .unwrap();

    let (hank_id, _) = client.create_document("/baseball", &up_content).unwrap().run().unwrap();

    // TODO: Make use of a Design type when available.

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert_object("views", |x| {
                             x.insert_object("home_runs", |x| {
                                 x.insert("map",
                                          "function(doc) { emit(doc.home_runs, doc.home_runs) }")
                             })
                         })
                         .unwrap();
    client.create_document("/baseball", &up_content)
          .unwrap()
          .with_document_id("_design/stats")
          .run()
          .unwrap();

    let expected = chill::testing::ViewResponseBuilder::new_unreduced(2, 0, "baseball")
                       .with_row(755, 755, hank_id)
                       .with_row(714, 714, babe_id)
                       .unwrap();

    let got = client.execute_view::<i32, i32, _>("/baseball/_design/stats/_view/home_runs")
                    .unwrap()
                    .with_descending(true)
                    .run()
                    .unwrap();

    assert_eq!(expected, got);
}

#[test]
fn execute_view_ok_reduced() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").unwrap().run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Babe Ruth")
                         .insert("home_runs", 714)
                         .unwrap();

    client.create_document("/baseball", &up_content).unwrap().run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert("name", "Hank Aaron")
                         .insert("home_runs", 755)
                         .unwrap();

    client.create_document("/baseball", &up_content).unwrap().run().unwrap();

    // TODO: Make use of a Design type when available.

    let up_content = serde_json::builder::ObjectBuilder::new()
                         .insert_object("views", |x| {
                             x.insert_object("home_runs", |x| {
                                 x.insert("map",
                                          r#"function(doc) { emit(doc.home_runs, doc.home_runs) }"#)
                                  .insert("reduce",
                                          r#"function(keys, values) {
                                               var c = 0;
                                               for (var i = 0; i < values.length; i++) {
                                                 c += values[i];
                                               }
                                               return c;
                                             }"#)
                             })
                         })
                         .unwrap();
    client.create_document("/baseball", &up_content)
          .unwrap()
          .with_document_id("_design/stats")
          .run()
          .unwrap();

    let expected = chill::testing::ViewResponseBuilder::new_reduced(714 + 755).unwrap();

    let got = client.execute_view::<(), i32, _>("/baseball/_design/stats/_view/home_runs")
                    .unwrap()
                    .run()
                    .unwrap();

    assert_eq!(expected, got);
}
