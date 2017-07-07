/*
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
    client.create_database("/baseball").run().unwrap();
}

#[test]
fn create_database_nok_database_exists() {
    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").run().unwrap();
    match client.create_database("/baseball").run() {
        Err(chill::Error::DatabaseExists(..)) => (),
        x @ _ => {
            panic!("Unexpected result: {:?}", x);
        }
    }
}

#[test]
fn create_document_ok_default() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert("nickname", "The Bambino")
        .build();

    let (doc_id, _rev) = client.create_document("/baseball", &up_content).run().unwrap();

    let doc = client.read_document(("/baseball", doc_id))
        .run()
        .unwrap();
    let down_content = doc.get_content().unwrap();
    assert_eq!(up_content, down_content);
}

#[test]
fn create_document_ok_with_document_id() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert("nickname", "The Bambino")
        .build();

    let (doc_id, _rev) = client.create_document("/baseball", &up_content)
        .with_document_id("babe_ruth")
        .run()
        .unwrap();
    assert_eq!(chill::DocumentId::from("babe_ruth"), doc_id);

    let doc = client.read_document(("/baseball", doc_id))
        .run()
        .unwrap();
    let down_content = doc.get_content().unwrap();
    assert_eq!(up_content, down_content);
}

#[test]
fn create_document_nok_document_conflict() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert("nickname", "The Bambino")
        .build();

    let (doc_id, _rev) = client.create_document("/baseball", &up_content).run().unwrap();

    match client.create_document("/baseball", &up_content)
        .with_document_id(doc_id)
        .run() {
        Err(chill::Error::DocumentConflict(..)) => (),
        x @ _ => unexpected_result!(x),
    }
}

#[test]
fn read_document_ok_default() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert("nickname", "The Bambino")
        .build();

    let (doc_id, _rev) = client.create_document("/baseball", &up_content).run().unwrap();

    let doc = client.read_document(("/baseball", doc_id))
        .run()
        .unwrap();
    let down_content = doc.get_content().unwrap();
    assert_eq!(up_content, down_content);
}

#[test]
fn read_document_ok_with_revision() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert("nickname", "The Bambino")
        .build();

    let (doc_id, rev) = client.create_document("/baseball", &up_content).run().unwrap();

    let doc = client.read_document(("/baseball", doc_id))
        .with_revision(&rev)
        .run()
        .unwrap();
    let down_content = doc.get_content().unwrap();
    assert_eq!(up_content, down_content);
}

#[test]
fn read_document_nok_not_found() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").run().unwrap();

    match client.read_document("/baseball/babe_ruth").run() {
        Err(chill::Error::NotFound(..)) => (),
        x @ _ => unexpected_result!(x),
    }
}


#[test]
fn read_document_ok_with_attachment_stubs() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").run().unwrap();

    // TODO: Create the attachment using strong types.

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert("nickname", "The Bambino")
        .insert_object("_attachments", |x| {
            x.insert_object("photo.png", |x| {
                x.insert("content_type", "image/png")
                    .insert("data",
                            base64::encode("Pretend this is a PNG file.".as_bytes()))
            })
        })
        .build();

    let (doc_id, _rev) = client.create_document("/baseball", &up_content).run().unwrap();

    let doc = client.read_document(("/baseball", doc_id))
        .run()
        .unwrap();

    let expected_attachments = {
        let mut m = std::collections::HashMap::<chill::AttachmentPath, (mime::Mime, u64)>::new();
        let path = chill::AttachmentPath::from((doc.path().clone(), chill::AttachmentName::from("photo.png")));
        m.insert(path,
                 (mime!(Image / Png), "Pretend this is a PNG file.".len() as u64));
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
    client.create_database("/baseball").run().unwrap();

    // TODO: Create the attachment using strong types.

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert("nickname", "The Bambino")
        .insert_object("_attachments", |x| {
            x.insert_object("photo.png", |x| {
                x.insert("content_type", "image/png")
                    .insert("data",
                            base64::encode("Pretend this is a PNG file.".as_bytes()))
            })
        })
        .build();

    let (doc_id, _rev) = client.create_document("/baseball", &up_content).run().unwrap();

    let doc = client.read_document(("/baseball", doc_id))
        .with_attachment_content(chill::action::read_document::AttachmentContent::All)
        .run()
        .unwrap();

    let expected_attachments = {
        let mut m = std::collections::HashMap::<chill::AttachmentPath, (mime::Mime, Vec<u8>)>::new();
        let path = chill::AttachmentPath::from((doc.path().clone(), chill::AttachmentName::from("photo.png")));
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
    client.create_database("/baseball").run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert("nickname", "The Bambino")
        .build();

    let (doc_id, _rev) = client.create_document("/baseball", &up_content).run().unwrap();

    let mut doc = client.read_document(("/baseball", doc_id))
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

    let updated_rev = client.update_document(&doc).run().unwrap();

    let doc = client.read_document(("/baseball", doc.path().document_id().clone()))
        .run()
        .unwrap();
    let down_content: serde_json::Value = doc.get_content().unwrap();
    assert_eq!(up_content, down_content);
    assert_eq!(&updated_rev, doc.revision());
}

#[test]
fn update_document_ok_create_attachment() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert("nickname", "The Bambino")
        .build();

    let (doc_id, _rev) = client.create_document("/baseball", &up_content).run().unwrap();

    let mut doc = client.read_document(("/baseball", doc_id.clone()))
        .run()
        .unwrap();

    doc.insert_attachment("photo.png",
                          mime!(Image / Png),
                          Vec::from("Pretend this is a PNG file."));

    client.update_document(&doc).run().unwrap();

    let doc = client.read_document(("/baseball", doc_id))
        .with_attachment_content(chill::action::read_document::AttachmentContent::All)
        .run()
        .unwrap();

    let expected_attachments = {
        let mut m = std::collections::HashMap::<chill::AttachmentPath, (mime::Mime, Vec<u8>)>::new();
        let path = chill::AttachmentPath::from((doc.path().clone(), chill::AttachmentName::from("photo.png")));
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
    client.create_database("/baseball").run().unwrap();

    // TODO: Create the attachment using strong types.

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert("nickname", "The Bambino")
        .insert_object("_attachments", |x| {
            x.insert_object("photo.png", |x| {
                x.insert("content_type", "image/png")
                    .insert("data",
                            base64::encode("Pretend this is a PNG file.".as_bytes()))
            })
        })
        .build();

    let (doc_id, _rev) = client.create_document("/baseball", &up_content).run().unwrap();

    let mut doc = client.read_document(("/baseball", doc_id.clone()))
        .run()
        .unwrap();

    doc.insert_attachment("photo.png",
                          mime!(Image / Png),
                          Vec::from("Pretend we updated the photo."));

    client.update_document(&doc).run().unwrap();

    let doc = client.read_document(("/baseball", doc_id))
        .with_attachment_content(chill::action::read_document::AttachmentContent::All)
        .run()
        .unwrap();

    let expected_attachments = {
        let mut m = std::collections::HashMap::<chill::AttachmentPath, (mime::Mime, Vec<u8>)>::new();
        let path = chill::AttachmentPath::from((doc.path().clone(), chill::AttachmentName::from("photo.png")));
        m.insert(path,
                 (mime!(Image / Png), Vec::from("Pretend we updated the photo.")));
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
    client.create_database("/baseball").run().unwrap();

    // TODO: Create the attachment using strong types.

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert("nickname", "The Bambino")
        .insert_object("_attachments", |x| {
            x.insert_object("photo.png", |x| {
                x.insert("content_type", "image/png")
                    .insert("data",
                            base64::encode("Pretend this is a PNG file.".as_bytes()))
            })
        })
        .build();

    let (doc_id, _rev) = client.create_document("/baseball", &up_content).run().unwrap();

    let mut doc = client.read_document(("/baseball", doc_id.clone()))
        .run()
        .unwrap();

    doc.remove_attachment("photo.png");

    client.update_document(&doc).run().unwrap();

    let doc = client.read_document(("/baseball", doc_id))
        .with_attachment_content(chill::action::read_document::AttachmentContent::All)
        .run()
        .unwrap();

    let expected_attachments = std::collections::HashMap::<chill::AttachmentPath, (mime::Mime, Vec<u8>)>::new();

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
    client.create_database("/baseball").run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert("nickname", "The Bambino")
        .build();

    let (doc_id, rev1) = client.create_document("/baseball", &up_content).run().unwrap();

    let rev2 = client.delete_document(("/baseball", doc_id.clone()), &rev1)
        .run()
        .unwrap();

    match client.read_document(("/baseball", doc_id.clone())).run() {
        Err(chill::Error::NotFound(..)) => (),
        x @ _ => {
            panic!("Unexpected result: {:?}", x);
        }
    }

    let doc = client.read_document(("/baseball", doc_id))
        .with_revision(&rev2)
        .run()
        .unwrap();
    assert!(doc.is_deleted());
}

#[test]
fn execute_view_ok_unreduced_default() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert("home_runs", 714)
        .build();

    let (babe_id, _) = client.create_document("/baseball", &up_content).run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Hank Aaron")
        .insert("home_runs", 755)
        .build();

    let (hank_id, _) = client.create_document("/baseball", &up_content).run().unwrap();

    let up_content = chill::DesignBuilder::new()
        .insert_view("home_runs",
                     chill::ViewFunction::new("function(doc) { emit(doc.home_runs, doc.home_runs) }"))
        .unwrap();

    client.create_document("/baseball", &up_content)
        .with_document_id("_design/stats")
        .run()
        .unwrap();

    let expected = chill::testing::ViewResponseBuilder::new_unreduced("baseball", 2, 0)
        .with_row(babe_id, 714, 714)
        .with_row(hank_id, 755, 755)
        .unwrap();

    let got = client.execute_view("/baseball/_design/stats/_view/home_runs")
        .run()
        .unwrap();

    assert_eq!(expected, got);
}

#[test]
fn execute_view_ok_unreduced_with_descending() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert("home_runs", 714)
        .build();

    let (babe_id, _) = client.create_document("/baseball", &up_content).run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Hank Aaron")
        .insert("home_runs", 755)
        .build();

    let (hank_id, _) = client.create_document("/baseball", &up_content).run().unwrap();

    let up_content = chill::DesignBuilder::new()
        .insert_view("home_runs",
                     chill::ViewFunction::new("function(doc) { emit(doc.home_runs, doc.home_runs) }"))
        .unwrap();

    client.create_document("/baseball", &up_content)
        .with_document_id("_design/stats")
        .run()
        .unwrap();

    let expected = chill::testing::ViewResponseBuilder::new_unreduced("baseball", 2, 0)
        .with_row(hank_id, 755, 755)
        .with_row(babe_id, 714, 714)
        .unwrap();

    let got = client.execute_view("/baseball/_design/stats/_view/home_runs")
        .with_descending(true)
        .run()
        .unwrap();

    assert_eq!(expected, got);
}

#[test]
fn execute_view_ok_unreduced_with_end_key_inclusive() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert("home_runs", 714)
        .build();

    let (babe_id, _) = client.create_document("/baseball", &up_content).run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Hank Aaron")
        .insert("home_runs", 755)
        .build();

    client.create_document("/baseball", &up_content).run().unwrap();

    let up_content = chill::DesignBuilder::new()
        .insert_view("home_runs",
                     chill::ViewFunction::new("function(doc) { emit(doc.home_runs, doc.home_runs) }"))
        .unwrap();

    client.create_document("/baseball", &up_content)
        .with_document_id("_design/stats")
        .run()
        .unwrap();

    let expected = chill::testing::ViewResponseBuilder::new_unreduced("baseball", 2, 0)
        .with_row(babe_id, 714, 714)
        .unwrap();

    let got = client.execute_view("/baseball/_design/stats/_view/home_runs")
        .with_end_key_inclusive(&714)
        .run()
        .unwrap();

    assert_eq!(expected, got);
}

#[test]
fn execute_view_ok_unreduced_with_end_key_exclusive() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert("home_runs", 714)
        .build();

    let (babe_id, _) = client.create_document("/baseball", &up_content).run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Hank Aaron")
        .insert("home_runs", 755)
        .build();

    client.create_document("/baseball", &up_content).run().unwrap();

    let up_content = chill::DesignBuilder::new()
        .insert_view("home_runs",
                     chill::ViewFunction::new("function(doc) { emit(doc.home_runs, doc.home_runs) }"))
        .unwrap();

    client.create_document("/baseball", &up_content)
        .with_document_id("_design/stats")
        .run()
        .unwrap();

    let expected = chill::testing::ViewResponseBuilder::new_unreduced("baseball", 2, 0)
        .with_row(babe_id, 714, 714)
        .unwrap();

    let got = client.execute_view("/baseball/_design/stats/_view/home_runs")
        .with_end_key_exclusive(&755)
        .run()
        .unwrap();

    assert_eq!(expected, got);
}

#[test]
fn execute_view_ok_unreduced_with_start_key() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert("home_runs", 714)
        .build();

    client.create_document("/baseball", &up_content).run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Hank Aaron")
        .insert("home_runs", 755)
        .build();

    let (hank_id, _) = client.create_document("/baseball", &up_content).run().unwrap();

    let up_content = chill::DesignBuilder::new()
        .insert_view("home_runs",
                     chill::ViewFunction::new("function(doc) { emit(doc.home_runs, doc.home_runs) }"))
        .unwrap();

    client.create_document("/baseball", &up_content)
        .with_document_id("_design/stats")
        .run()
        .unwrap();

    let expected = chill::testing::ViewResponseBuilder::new_unreduced("baseball", 2, 1)
        .with_row(hank_id, 755, 755)
        .unwrap();

    let got = client.execute_view("/baseball/_design/stats/_view/home_runs")
        .with_start_key(&730)
        .run()
        .unwrap();

    assert_eq!(expected, got);
}

#[test]
fn execute_view_ok_unreduced_with_reduce_false() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert("home_runs", 714)
        .build();

    let (babe_id, _rev) = client.create_document("/baseball", &up_content).run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Hank Aaron")
        .insert("home_runs", 755)
        .build();

    let (hank_id, _rev) = client.create_document("/baseball", &up_content).run().unwrap();

    let up_content = chill::DesignBuilder::new()
        .insert_view("home_runs",
                     chill::ViewFunction::new_with_reduce("function(doc) { emit(doc.home_runs, doc.home_runs) }",
                                                          r#"function(keys, values) {
                                               var c = 0;
                                               for (var i = 0; i < values.length; i++) {
                                                 c += values[i];
                                               }
                                               return c;
                                             }"#))
        .unwrap();

    client.create_document("/baseball", &up_content)
        .with_document_id("_design/stats")
        .run()
        .unwrap();

    let expected = chill::testing::ViewResponseBuilder::new_unreduced("baseball", 2, 0)
        .with_row(babe_id, 714, 714)
        .with_row(hank_id, 755, 755)
        .unwrap();

    let got = client.execute_view("/baseball/_design/stats/_view/home_runs")
        .with_reduce(false)
        .run()
        .unwrap();

    assert_eq!(expected, got);
}

#[test]
fn execute_view_ok_unreduced_with_limit() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert("home_runs", 714)
        .build();

    let (babe_id, _rev) = client.create_document("/baseball", &up_content).run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Hank Aaron")
        .insert("home_runs", 755)
        .build();

    client.create_document("/baseball", &up_content).run().unwrap();

    let up_content = chill::DesignBuilder::new()
        .insert_view("home_runs",
                     chill::ViewFunction::new("function(doc) { emit(doc.home_runs, doc.home_runs) }"))
        .unwrap();

    client.create_document("/baseball", &up_content)
        .with_document_id("_design/stats")
        .run()
        .unwrap();

    let expected = chill::testing::ViewResponseBuilder::new_unreduced("baseball", 2, 0)
        .with_row(babe_id, 714, 714)
        .unwrap();

    let got = client.execute_view("/baseball/_design/stats/_view/home_runs")
        .with_limit(1)
        .run()
        .unwrap();

    assert_eq!(expected, got);
}

#[test]
fn execute_view_ok_unreduced_with_documents() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").run().unwrap();

    let babe_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert("home_runs", 714)
        .build();

    let (babe_id, babe_rev) = client.create_document("/baseball", &babe_content).run().unwrap();

    let hank_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Hank Aaron")
        .insert("home_runs", 755)
        .build();

    let (hank_id, hank_rev) = client.create_document("/baseball", &hank_content).run().unwrap();

    let design_content = chill::DesignBuilder::new()
        .insert_view("home_runs",
                     chill::ViewFunction::new("function(doc) { emit(doc.home_runs, doc.home_runs) }"))
        .unwrap();

    client.create_document("/baseball", &design_content)
        .with_document_id("_design/stats")
        .run()
        .unwrap();

    let expected = chill::testing::ViewResponseBuilder::new_unreduced("baseball", 2, 0)
        .with_row_with_document(babe_id.clone(),
                                714,
                                714,
                                chill::testing::DocumentBuilder::new(("/baseball", babe_id), babe_rev)
                                    .with_content(&babe_content)
                                    .unwrap())
        .with_row_with_document(hank_id.clone(),
                                755,
                                755,
                                chill::testing::DocumentBuilder::new(("/baseball", hank_id), hank_rev)
                                    .with_content(&hank_content)
                                    .unwrap())
        .unwrap();

    let got = client.execute_view("/baseball/_design/stats/_view/home_runs")
        .with_documents(true)
        .run()
        .unwrap();

    assert_eq!(expected, got);
}

#[test]
fn execute_view_ok_reduced() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert("home_runs", 714)
        .build();

    client.create_document("/baseball", &up_content).run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Hank Aaron")
        .insert("home_runs", 755)
        .build();

    client.create_document("/baseball", &up_content).run().unwrap();

    let up_content = chill::DesignBuilder::new()
        .insert_view("home_runs",
                     chill::ViewFunction::new_with_reduce(r#"function(doc) { emit(doc.home_runs, doc.home_runs) }"#,
                                                          r#"function(keys, values) {
                                               var c = 0;
                                               for (var i = 0; i < values.length; i++) {
                                                 c += values[i];
                                               }
                                               return c;
                                             }"#))
        .unwrap();

    client.create_document("/baseball", &up_content)
        .with_document_id("_design/stats")
        .run()
        .unwrap();

    let expected = chill::testing::ViewResponseBuilder::new_reduced(714 + 755).unwrap();

    let got = client.execute_view("/baseball/_design/stats/_view/home_runs")
        .run()
        .unwrap();

    assert_eq!(expected, got);
}

#[test]
fn execute_view_ok_grouped_exact() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert_object("home_runs", |x| {
            x.insert("1919", 29)
                .insert("1920", 54)
                .insert("1921", 59)
        })
        .build();

    client.create_document("/baseball", &up_content).run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Hank Aaron")
        .insert_object("home_runs", |x| {
            x.insert("1966", 44)
                .insert("1967", 39)
                .insert("1968", 29)
        })
        .build();

    client.create_document("/baseball", &up_content).run().unwrap();

    let up_content = chill::DesignBuilder::new()
        .insert_view("home_runs",
                     chill::ViewFunction::new_with_reduce(r#"function(doc) {
                                               for (year in doc.home_runs) {
                                                 emit([doc.home_runs[year], doc.name], 1);
                                               }
                                             }"#,
                                                          "_sum"))
        .unwrap();

    client.create_document("/baseball", &up_content)
        .with_document_id("_design/stats")
        .run()
        .unwrap();

    let new_key = |hr, name| {
        serde_json::builder::ArrayBuilder::new()
            .push(hr)
            .push(name)
            .build()
    };

    let expected = chill::testing::ViewResponseBuilder::new_grouped()
        .with_row(new_key(29, "Babe Ruth"), 1)
        .with_row(new_key(29, "Hank Aaron"), 1)
        .with_row(new_key(39, "Hank Aaron"), 1)
        .with_row(new_key(44, "Hank Aaron"), 1)
        .with_row(new_key(54, "Babe Ruth"), 1)
        .with_row(new_key(59, "Babe Ruth"), 1)
        .unwrap();

    let got = client.execute_view("/baseball/_design/stats/_view/home_runs")
        .with_exact_groups(true)
        .run()
        .unwrap();

    assert_eq!(expected, got);
}

#[test]
fn execute_view_ok_grouped_with_level() {

    let (_server, client) = make_server_and_client();
    client.create_database("/baseball").run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Babe Ruth")
        .insert_object("home_runs", |x| {
            x.insert("1919", 29)
                .insert("1920", 54)
                .insert("1921", 59)
        })
        .build();

    client.create_document("/baseball", &up_content).run().unwrap();

    let up_content = serde_json::builder::ObjectBuilder::new()
        .insert("name", "Hank Aaron")
        .insert_object("home_runs", |x| {
            x.insert("1966", 44)
                .insert("1967", 39)
                .insert("1968", 29)
        })
        .build();

    client.create_document("/baseball", &up_content).run().unwrap();

    let up_content = chill::DesignBuilder::new()
        .insert_view("home_runs",
                     chill::ViewFunction::new_with_reduce(r#"function(doc) {
                                               for (year in doc.home_runs) {
                                                 emit([doc.home_runs[year], doc.name], 1);
                                               }
                                             }"#,
                                                          "_sum"))
        .unwrap();

    client.create_document("/baseball", &up_content)
        .with_document_id("_design/stats")
        .run()
        .unwrap();

    let new_key = |hr| {
        serde_json::builder::ArrayBuilder::new()
            .push(hr)
            .build()
    };

    let expected = chill::testing::ViewResponseBuilder::new_grouped()
        .with_row(new_key(29), 2)
        .with_row(new_key(39), 1)
        .with_row(new_key(44), 1)
        .with_row(new_key(54), 1)
        .with_row(new_key(59), 1)
        .unwrap();

    let got = client.execute_view("/baseball/_design/stats/_view/home_runs")
        .with_group_level(1)
        .run()
        .unwrap();

    assert_eq!(expected, got);
}
*/
