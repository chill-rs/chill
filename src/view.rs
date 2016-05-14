use {DatabaseName, DocumentId, DocumentPath, Error};
use {serde, serde_json, std};

#[derive(Clone, Debug, PartialEq)]
pub struct ViewResponse {
    total_rows: Option<u64>,
    offset: Option<u64>,
    rows: Vec<ViewRow>,
    update_seq: Option<u64>,
}

impl ViewResponse {
    #[doc(hidden)]
    pub fn new_from_decoded(db_name: DatabaseName, decoded: ViewResponseJsonable) -> Self {

        let rows = decoded.rows
            .into_iter()
            .map(|x| ViewRow::new_from_decoded(db_name.clone(), x))
            .collect();

        // let rows = try!(decoded.rows
        //                         .into_iter()
        //                         .map(|x| ViewRow::new_from_decoded(db_name.clone(), x))
        //                         .collect::<Vec<_>>()
        //                         .into_iter()
        //                         .collect());

        ViewResponse {
            total_rows: decoded.total_rows,
            offset: decoded.offset,
            rows: rows,
            update_seq: decoded.update_seq,
        }
    }


    /// Returns how many rows are in the view, including rows excluded in the
    /// response, if available.
    ///
    /// The total number of rows is available if and only if the view is
    /// unreduced. Even group-reduced views, which may contain multiple rows in
    /// the response, have no total number of rows associated with them.
    ///
    pub fn total_rows(&self) -> Option<u64> {
        self.total_rows
    }

    /// Returns how many rows are excluded from the view response that are
    /// ordered before the first row in the response, if available.
    ///
    /// The offset is available if and only if the view is unreduced.
    ///
    pub fn offset(&self) -> Option<u64> {
        self.offset
    }

    /// Returns the update sequence number that the view reflects, if available.
    pub fn update_sequence_number(&self) -> Option<u64> {
        self.update_seq
    }

    /// Returns the vector containing all rows in the view response.
    pub fn rows(&self) -> &Vec<ViewRow> {
        &self.rows
    }
}

#[doc(hidden)]
impl Default for ViewResponse {
    fn default() -> Self {
        ViewResponse {
            total_rows: None,
            offset: None,
            rows: Vec::new(),
            update_seq: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ViewRow {
    key: Option<serde_json::Value>,
    value: serde_json::Value,
    doc_path: Option<DocumentPath>,
}

impl ViewRow {
    fn new_from_decoded(db_name: DatabaseName, decoded: ViewRowJsonable) -> Self {

        let doc_path = decoded.id.map(|doc_id| DocumentPath::from((db_name, doc_id)));

        ViewRow {
            key: decoded.key,
            value: decoded.value,
            doc_path: doc_path,
        }
    }

    /// Returns the row's key, if available.
    ///
    /// The key is available if and only if the view is unreduced or if the view
    /// is reduced but grouped.
    ///
    pub fn key<K: serde::Deserialize>(&self) -> Result<Option<K>, Error> {

        let decoded = match self.key {
            None => None,
            Some(ref key) => {
                // FIXME: Optimize this to eliminate cloning and re-decoding.
                try!(serde_json::from_value(key.clone())
                         .map_err(|e| Error::JsonDecode { cause: e }))
            }
        };

        Ok(decoded)
    }

    /// Returns the row's value.
    pub fn value<V: serde::Deserialize>(&self) -> Result<V, Error> {
        // FIXME: Optimize this to eliminate cloning and re-decoding.
        serde_json::from_value(self.value.clone()).map_err(|e| Error::JsonDecode { cause: e })
    }

    /// Returns the row's related document path, if available.
    ///
    /// The document path is available if and only if the view is unreduced.
    ///
    pub fn document_path(&self) -> Option<&DocumentPath> {
        self.doc_path.as_ref()
    }
}

#[derive(Debug, PartialEq)]
pub struct ViewResponseJsonable {
    total_rows: Option<u64>,
    offset: Option<u64>,
    update_seq: Option<u64>,
    rows: Vec<ViewRowJsonable>,
}

impl serde::Deserialize for ViewResponseJsonable {
    fn deserialize<D: serde::Deserializer>(deserializer: &mut D) -> Result<Self, D::Error> {
        enum Field {
            Offset,
            Rows,
            TotalRows,
            UpdateSeq,
        }

        impl serde::Deserialize for Field {
            fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
                where D: serde::Deserializer
            {
                struct Visitor;

                impl serde::de::Visitor for Visitor {
                    type Value = Field;

                    fn visit_str<E>(&mut self, value: &str) -> Result<Self::Value, E>
                        where E: serde::de::Error
                    {
                        match value {
                            "offset" => Ok(Field::Offset),
                            "rows" => Ok(Field::Rows),
                            "total_rows" => Ok(Field::TotalRows),
                            "update_seq" => Ok(Field::UpdateSeq),
                            _ => Err(E::unknown_field(value)),
                        }
                    }
                }

                deserializer.deserialize(Visitor)
            }
        }

        struct Visitor;

        impl serde::de::Visitor for Visitor {
            type Value = ViewResponseJsonable;

            fn visit_map<Vis>(&mut self, mut visitor: Vis) -> Result<Self::Value, Vis::Error>
                where Vis: serde::de::MapVisitor
            {
                let mut offset = None;
                let mut rows = None;
                let mut total_rows = None;
                let mut update_seq = None;

                loop {
                    match try!(visitor.visit_key()) {
                        Some(Field::Offset) => {
                            offset = Some(try!(visitor.visit_value()));
                        }
                        Some(Field::Rows) => {
                            rows = Some(try!(visitor.visit_value()));
                        }
                        Some(Field::TotalRows) => {
                            total_rows = Some(try!(visitor.visit_value()));
                        }
                        Some(Field::UpdateSeq) => {
                            update_seq = Some(try!(visitor.visit_value()));
                        }
                        None => {
                            break;
                        }
                    }
                }

                try!(visitor.end());

                let rows = match rows {
                    Some(x) => x,
                    None => try!(visitor.missing_field("rows")),
                };

                Ok(ViewResponseJsonable {
                    total_rows: total_rows,
                    offset: offset,
                    update_seq: update_seq,
                    rows: rows,
                })
            }
        }

        static FIELDS: &'static [&'static str] = &["total_rows", "offset", "rows", "update_seq"];
        deserializer.deserialize_struct("ViewResponseJsonable", FIELDS, Visitor)
    }
}

#[derive(Debug, PartialEq)]
struct ViewRowJsonable {
    key: Option<serde_json::Value>,
    value: serde_json::Value,
    id: Option<DocumentId>,
}

impl serde::Deserialize for ViewRowJsonable {
    fn deserialize<D: serde::Deserializer>(deserializer: &mut D) -> Result<Self, D::Error> {
        enum Field {
            Id,
            Key,
            Value,
        }

        impl serde::Deserialize for Field {
            fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
                where D: serde::Deserializer
            {
                struct Visitor;

                impl serde::de::Visitor for Visitor {
                    type Value = Field;

                    fn visit_str<E>(&mut self, value: &str) -> Result<Self::Value, E>
                        where E: serde::de::Error
                    {
                        match value {
                            "id" => Ok(Field::Id),
                            "key" => Ok(Field::Key),
                            "value" => Ok(Field::Value),
                            _ => Err(E::unknown_field(value)),
                        }
                    }
                }

                deserializer.deserialize(Visitor)
            }
        }

        struct Visitor;

        impl serde::de::Visitor for Visitor {
            type Value = ViewRowJsonable;

            fn visit_map<Vis>(&mut self, mut visitor: Vis) -> Result<Self::Value, Vis::Error>
                where Vis: serde::de::MapVisitor
            {
                let mut id = None;
                let mut key = None;
                let mut value = None;

                loop {
                    match try!(visitor.visit_key()) {
                        Some(Field::Id) => {
                            id = Some(try!(visitor.visit_value()));
                        }
                        Some(Field::Key) => {
                            key = try!(visitor.visit_value()); // allow null
                        }
                        Some(Field::Value) => {
                            value = Some(try!(visitor.visit_value()));
                        }
                        None => {
                            break;
                        }
                    }
                }

                try!(visitor.end());

                let value = match value {
                    Some(x) => x,
                    None => try!(visitor.missing_field("value")),
                };

                Ok(ViewRowJsonable {
                    key: key,
                    value: value,
                    id: id,
                })
            }
        }

        static FIELDS: &'static [&'static str] = &["id", "key", "value"];
        deserializer.deserialize_struct("ViewRowJsonable", FIELDS, Visitor)
    }
}

pub struct IsReduced;
pub struct IsGrouped;
pub struct IsUnreduced;

#[derive(Debug)]
pub struct ViewResponseBuilder<T> {
    phantom: std::marker::PhantomData<T>,
    db_name: Option<DatabaseName>,
    target: ViewResponse,
}

impl ViewResponseBuilder<IsReduced> {
    pub fn new_reduced<V: serde::Serialize>(value: V) -> Self {
        ViewResponseBuilder {
            phantom: std::marker::PhantomData,
            db_name: None,
            target: ViewResponse {
                rows: vec![ViewRow {
                               key: None,
                               value: serde_json::to_value(&value),
                               doc_path: None,
                           }],
                ..ViewResponse::default()
            },
        }
    }

    pub fn new_reduced_empty() -> Self {
        ViewResponseBuilder {
            phantom: std::marker::PhantomData,
            db_name: None,
            target: ViewResponse::default(),
        }
    }
}

impl ViewResponseBuilder<IsGrouped> {
    pub fn new_grouped() -> Self {
        ViewResponseBuilder {
            phantom: std::marker::PhantomData,
            db_name: None,
            target: ViewResponse { rows: Vec::new(), ..ViewResponse::default() },
        }
    }

    pub fn with_row<K, V>(mut self, key: K, value: V) -> Self
        where K: serde::Serialize,
              V: serde::Serialize
    {

        self.target.rows.push(ViewRow {
            key: Some(serde_json::to_value(&key)),
            value: serde_json::to_value(&value),
            doc_path: None,
        });

        self
    }
}

impl ViewResponseBuilder<IsUnreduced> {
    pub fn new_unreduced<D>(db_name: D, total_rows: u64, offset: u64) -> Self
        where D: Into<DatabaseName>
    {
        ViewResponseBuilder {
            phantom: std::marker::PhantomData,
            db_name: Some(db_name.into()),
            target: ViewResponse {
                total_rows: Some(total_rows),
                offset: Some(offset),
                update_seq: None,
                rows: Vec::new(),
            },
        }
    }

    pub fn with_row<D, K, V>(mut self, doc_id: D, key: K, value: V) -> Self
        where D: Into<DocumentId>,
              K: serde::Serialize,
              V: serde::Serialize
    {
        self.target.rows.push(ViewRow {
            key: Some(serde_json::to_value(&key)),
            value: serde_json::to_value(&value),
            doc_path: Some(DocumentPath::from((self.db_name.as_ref().unwrap().clone(), doc_id.into()))),
        });

        self
    }
}

impl<T> ViewResponseBuilder<T> {
    /// Sets the update sequence number for the view response.
    ///
    /// By default, the view response's update sequence number is `None`.
    ///
    pub fn with_update_sequence_number(mut self, update_seq: u64) -> Self {
        self.target.update_seq = Some(update_seq);
        self
    }

    /// Returns the builder's view response.
    pub fn unwrap(self) -> ViewResponse {
        self.target
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use super::ViewRowJsonable;
    use {DocumentId, Error, IntoDocumentPath};
    use serde_json;

    #[test]
    fn view_row_key_ok_none() {

        let row = ViewRow {
            key: None,
            value: serde_json::Value::U64(42),
            doc_path: None,
        };

        let got = row.key::<String>().unwrap();
        assert_eq!(None, got);
    }

    #[test]
    fn view_row_key_ok_some() {

        let row = ViewRow {
            key: Some(serde_json::Value::String(String::from("foo"))),
            value: serde_json::Value::U64(42),
            doc_path: Some("/db/doc".into_document_path().unwrap()),
        };

        let expected = Some(String::from("foo"));
        let got = row.key::<String>().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn view_row_key_nok() {

        let row = ViewRow {
            key: Some(serde_json::Value::String(String::from("foo"))),
            value: serde_json::Value::U64(42),
            doc_path: Some("/db/doc".into_document_path().unwrap()),
        };

        match row.key::<u64>() {
            Err(Error::JsonDecode { .. }) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn view_row_value_ok() {

        let row = ViewRow {
            key: None,
            value: serde_json::Value::U64(42),
            doc_path: None,
        };

        let expected: u64 = 42;
        let got = row.value::<u64>().unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn view_row_value_nok() {

        let row = ViewRow {
            key: None,
            value: serde_json::Value::U64(42),
            doc_path: None,
        };

        match row.value::<String>() {
            Err(Error::JsonDecode { .. }) => (),
            x @ _ => unexpected_result!(x),
        }
    }

    #[test]
    fn view_row_deserialize_ok_reduced() {

        let expected = ViewRowJsonable {
            id: None,
            key: None,
            value: serde_json::Value::U64(42),
        };

        let json_text = r#"{"key": null, "value": 42}"#;

        let got = serde_json::from_str(&json_text).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn view_row_deserialize_ok_unreduced() {

        let expected = ViewRowJsonable {
            id: Some(DocumentId::from("foo")),
            key: Some(serde_json::Value::String(String::from("bar"))),
            value: serde_json::Value::U64(42),
        };

        let json_text = r#"{"id": "foo", "key": "bar", "value": 42}"#;

        let got = serde_json::from_str(&json_text).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn view_row_deserialize_nok_missing_value() {

        let json_text = r#"{"id": "foo", "key": "bar"}"#;

        let got = serde_json::from_str::<ViewRowJsonable>(&json_text);
        expect_json_error_missing_field!(got, "value");
    }

    #[test]
    fn view_response_deserialize_ok_reduced() {

        let expected = ViewResponseJsonable {
            total_rows: None,
            offset: None,
            update_seq: None,
            rows: vec![ViewRowJsonable {
                           id: None,
                           key: None,
                           value: serde_json::Value::U64(42),
                       }],
        };

        let json_text = r#"{"rows": [
            {"key": null, "value": 42}
            ]}"#;

        let got = serde_json::from_str(&json_text).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn view_response_deserialize_ok_reduced_with_update_seq() {

        let expected = ViewResponseJsonable {
            total_rows: None,
            offset: None,
            update_seq: Some(17),
            rows: vec![ViewRowJsonable {
                           id: None,
                           key: None,
                           value: serde_json::Value::U64(42),
                       }],
        };

        let json_text = r#"{"update_seq": 17, "rows": [
            {"key": null, "value": 42}
            ]}"#;

        let got = serde_json::from_str(&json_text).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn view_response_deserialize_ok_reduced_grouped() {

        let expected = ViewResponseJsonable {
            total_rows: None,
            offset: None,
            update_seq: None,
            rows: vec![ViewRowJsonable {
                           id: None,
                           key: Some(serde_json::builder::ArrayBuilder::new()
                                         .push(1)
                                         .push(2)
                                         .unwrap()),
                           value: serde_json::Value::U64(42),
                       },
                       ViewRowJsonable {
                           id: None,
                           key: Some(serde_json::builder::ArrayBuilder::new()
                                         .push(1)
                                         .push(3)
                                         .unwrap()),
                           value: serde_json::Value::U64(43),
                       },
                       ViewRowJsonable {
                           id: None,
                           key: Some(serde_json::builder::ArrayBuilder::new()
                                         .push(2)
                                         .push(3)
                                         .unwrap()),
                           value: serde_json::Value::U64(44),
                       }],
        };

        let json_text = r#"{"rows":[
            {"key":[1,2],"value":42},
            {"key":[1,3],"value":43},
            {"key":[2,3],"value":44}
            ]}"#;

        let got = serde_json::from_str(&json_text).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn view_response_deserialize_ok_unreduced() {

        let expected = ViewResponseJsonable {
            total_rows: Some(10),
            offset: Some(5),
            update_seq: None,
            rows: vec![ViewRowJsonable {
                           id: Some(DocumentId::from("foo")),
                           key: Some(serde_json::Value::String(String::from("bar"))),
                           value: serde_json::Value::U64(42),
                       },
                       ViewRowJsonable {
                           id: Some(DocumentId::from("qux")),
                           key: Some(serde_json::Value::String(String::from("baz"))),
                           value: serde_json::Value::U64(17),
                       },
            ],
        };

        let json_text = r#"{"total_rows": 10, "offset": 5, "rows": [
            {"id": "foo", "key": "bar", "value": 42},
            {"id": "qux", "key": "baz", "value": 17}
            ]}"#;

        let got = serde_json::from_str(&json_text).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn view_response_deserialize_ok_unreduced_with_update_seq() {

        let expected = ViewResponseJsonable {
            total_rows: Some(10),
            offset: Some(5),
            update_seq: Some(17),
            rows: vec![ViewRowJsonable {
                           id: Some(DocumentId::from("foo")),
                           key: Some(serde_json::Value::String(String::from("bar"))),
                           value: serde_json::Value::U64(42),
                       }],
        };

        let json_text = r#"{"total_rows": 10, "offset": 5, "update_seq": 17, "rows": [
            {"id": "foo", "key": "bar", "value": 42}
            ]}"#;

        let got = serde_json::from_str(&json_text).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn view_response_builder_reduced() {

        let expected = ViewResponse {
            total_rows: None,
            offset: None,
            update_seq: Some(99),
            rows: vec![ViewRow {
                           key: None,
                           value: serde_json::Value::U64(42),
                           doc_path: None,
                       }],
        };

        let got = ViewResponseBuilder::new_reduced(42).with_update_sequence_number(99).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn view_response_builder_reduced_empty() {

        let expected = ViewResponse {
            total_rows: None,
            offset: None,
            update_seq: Some(99),
            rows: Vec::new(),
        };

        let got = ViewResponseBuilder::new_reduced_empty().with_update_sequence_number(99).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn view_response_builder_grouped() {

        let expected = ViewResponse {
            total_rows: None,
            offset: None,
            update_seq: Some(99),
            rows: vec![ViewRow {
                           key: Some(serde_json::Value::Array(vec![serde_json::Value::U64(1)])),
                           value: serde_json::Value::String(String::from("alpha")),
                           doc_path: None,
                       },
                       ViewRow {
                           key: Some(serde_json::Value::Array(vec![serde_json::Value::U64(2)])),
                           value: serde_json::Value::String(String::from("bravo")),
                           doc_path: None,
                       }],
        };

        let got = ViewResponseBuilder::new_grouped()
            .with_update_sequence_number(99)
            .with_row(vec![1], "alpha")
            .with_row(vec![2], "bravo")
            .unwrap();

        assert_eq!(expected, got);
    }

    #[test]
    fn view_response_builder_unreduced() {

        let expected = ViewResponse {
            total_rows: Some(42),
            offset: Some(17),
            update_seq: Some(99),
            rows: vec![ViewRow {
                           key: Some(serde_json::Value::U64(1)),
                           value: serde_json::Value::String(String::from("bravo")),
                           doc_path: Some("/db/alpha".into_document_path().unwrap()),
                       },
                       ViewRow {
                           key: Some(serde_json::Value::U64(2)),
                           value: serde_json::Value::String(String::from("delta")),
                           doc_path: Some("/db/charlie".into_document_path().unwrap()),
                       }],
        };

        let got = ViewResponseBuilder::new_unreduced("db", 42, 17)
            .with_update_sequence_number(99)
            .with_row("alpha", 1, "bravo")
            .with_row("charlie", 2, "delta")
            .unwrap();

        assert_eq!(expected, got);
    }
}
