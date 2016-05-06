use {DatabaseName, DocumentId, DocumentPath, Error};
use {serde, std};

/// Contains the response from the CouchDB server as a result of successfully
/// executing a view.
///
/// A `ViewResponse` takes one of two forms, **reduced** or **unreduced**,
/// depending on whether the view's <q>reduce</q> function ran. See the CouchDB
/// documentation for more details.
///
/// Although `ViewResponse` implements the `Ord` and `PartialOrd` traits, Chill
/// makes no guarantee how that ordering is defined and may change the
/// definition in an upcoming release. Chill defines the ordering only so that
/// applications may use `ViewResponse` in ordered collections such as trees.
///
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ViewResponse<K: serde::Deserialize, V: serde::Deserialize> {
    /// Contains a reduced view response.
    Reduced(ReducedView<V>),

    /// Contains an unreduced view response.
    Unreduced(UnreducedView<K, V>),
}

impl<K: serde::Deserialize, V: serde::Deserialize> ViewResponse<K, V> {
    #[doc(hidden)]
    pub fn new_from_decoded(db_name: DatabaseName,
                            mut response: ViewResponseJsonable<K, V>)
                            -> Result<Self, Error> {
        if 1 == response.rows.len() && response.total_rows.is_none() && response.offset.is_none() {
            Ok(ViewResponse::Reduced(ReducedView {
                update_seq: response.update_seq,
                value: response.rows.pop().unwrap().value,
            }))
        } else {

            let total_rows = match response.total_rows {
                Some(x) => x,
                None => {
                    return Err(Error::UnexpectedResponse("missing 'total_rows' field"));
                }
            };

            let offset = match response.offset {
                Some(x) => x,
                None => {
                    return Err(Error::UnexpectedResponse("missing 'offset' field"));
                }
            };

            let rows = try!(response.rows
                                    .into_iter()
                                    .map(|x| ViewRow::new_from_decoded(db_name.clone(), x))
                                    .collect::<Vec<_>>()
                                    .into_iter()
                                    .collect());

            Ok(ViewResponse::Unreduced(UnreducedView {
                total_rows: total_rows,
                offset: offset,
                update_seq: response.update_seq,
                rows: rows,
            }))
        }
    }

    /// Returns the view response in its reduced form, if the response is
    /// reduced.
    pub fn as_reduced(&self) -> Option<&ReducedView<V>> {
        match self {
            &ViewResponse::Reduced(ref x) => Some(x),
            _ => None,
        }
    }

    /// Returns the view response in its reduced form, if the response is
    /// reduced.
    pub fn as_reduced_mut(&mut self) -> Option<&mut ReducedView<V>> {
        match self {
            &mut ViewResponse::Reduced(ref mut x) => Some(x),
            _ => None,
        }
    }

    /// Returns the view response in its unreduced form, if the response is
    /// unreduced.
    pub fn as_unreduced(&self) -> Option<&UnreducedView<K, V>> {
        match self {
            &ViewResponse::Unreduced(ref x) => Some(x),
            _ => None,
        }
    }

    /// Returns the view response in its unreduced form, if the response is
    /// unreduced.
    pub fn as_unreduced_mut(&mut self) -> Option<&mut UnreducedView<K, V>> {
        match self {
            &mut ViewResponse::Unreduced(ref mut x) => Some(x),
            _ => None,
        }
    }
}

#[cfg(test)]
mod view_response_tests {

    use super::*;

    #[test]
    fn impls_send() {
        fn f<T: Send>(_: T) {}
        f(ViewResponse::Reduced::<(), i32>(ReducedView {
            update_seq: None,
            value: 42,
        }));
    }
}

/// Contains a view response in reduced form.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ReducedView<V: serde::Deserialize> {
    update_seq: Option<u64>,
    value: V,
}

impl<V: serde::Deserialize> ReducedView<V> {
    /// Returns the view response's reduced value.
    pub fn value(&self) -> &V {
        &self.value
    }

    /// Returns the update sequence number the view reflects, if available.
    pub fn update_sequence_number(&self) -> Option<u64> {
        self.update_seq
    }
}

/// Contains a view response in unreduced form.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UnreducedView<K: serde::Deserialize, V: serde::Deserialize> {
    total_rows: u64,
    offset: u64,
    update_seq: Option<u64>,
    rows: Vec<ViewRow<K, V>>,
}

impl<K: serde::Deserialize, V: serde::Deserialize> UnreducedView<K, V> {
    /// Returns the number of all rows in the view response, including rows
    /// excluded from the vector.
    pub fn total_rows(&self) -> u64 {
        self.total_rows
    }

    /// Returns the number of rows in the view response excluded before the first
    /// row in the vector.
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// Returns the update sequence number the view reflects, if available.
    pub fn update_sequence_number(&self) -> Option<u64> {
        self.update_seq
    }

    /// Returns the vector containing all rows in the view response.
    pub fn rows(&self) -> &Vec<ViewRow<K, V>> {
        &self.rows
    }
}

/// Marks that a view is reduced.
///
/// `ViewIsReduced` is a marker type that applications should not need to
/// explicitly use. The Rust compiler should infer this type where appropriate.
///
pub struct ViewIsReduced;

/// Marks that a view is unreduced.
///
/// `ViewIsUnreduced` is a marker type that applications should not need to
/// explicitly use. The Rust compiler should infer this type where appropriate.
///
pub struct ViewIsUnreduced;

/// Contains a single row in an unreduced view response.
///
/// See the CouchDB documentation for more details about view rows.
///
/// Although `ViewRow` implements the `Ord` and `PartialOrd` traits, Chill makes
/// no guarantee how that ordering is defined and may change the definition in
/// an upcoming release. One consequence is that if an application sorts view
/// rows itself then the result may be in a different order than if the CouchDB
/// server had sorted the rows. This is an anti-pattern; applications should
/// rely on the server to do sorting. Chill defines the ordering only so that
/// applications may use `ViewRow` in ordered collections such as trees.
///
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ViewRow<K: serde::Deserialize, V: serde::Deserialize> {
    key: K,
    value: V,
    doc_path: DocumentPath,
}

impl<K, V> ViewRow<K, V>
    where K: serde::Deserialize,
          V: serde::Deserialize
{
    fn new_from_decoded(db_name: DatabaseName, row: ViewRowJsonable<K, V>) -> Result<Self, Error> {

        let key = match row.key {
            Some(x) => x,
            None => {
                return Err(Error::UnexpectedResponse("missing key in row"));
            }
        };

        let doc_id = match row.id {
            Some(x) => x,
            None => {
                return Err(Error::UnexpectedResponse("missing document id in row"));
            }
        };

        Ok(ViewRow {
            key: key,
            value: row.value,
            doc_path: DocumentPath::from((db_name, doc_id)),
        })
    }

    /// Returns the row's key.
    pub fn key(&self) -> &K {
        &self.key
    }

    /// Returns the row's value.
    pub fn value(&self) -> &V {
        &self.value
    }

    /// Returns the path of the row's document.
    pub fn document_path(&self) -> &DocumentPath {
        &self.doc_path
    }
}

#[derive(Debug, PartialEq)]
pub struct ViewResponseJsonable<K: serde::Deserialize, V: serde::Deserialize> {
    total_rows: Option<u64>,
    offset: Option<u64>,
    update_seq: Option<u64>,
    rows: Vec<ViewRowJsonable<K, V>>,
}

impl<K, V> serde::Deserialize for ViewResponseJsonable<K, V>
    where K: serde::Deserialize,
          V: serde::Deserialize
{
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: serde::Deserializer
    {
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

        struct Visitor<K2, V2>
            where K2: serde::Deserialize,
                  V2: serde::Deserialize
        {
            _phantom_key: std::marker::PhantomData<K2>,
            _phantom_value: std::marker::PhantomData<V2>,
        }

        impl<K2, V2> serde::de::Visitor for Visitor<K2, V2>
            where K2: serde::Deserialize,
                  V2: serde::Deserialize
{
            type Value = ViewResponseJsonable<K2, V2>;

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
        deserializer.deserialize_struct("ViewResponseJsonable",
                                        FIELDS,
                                        Visitor::<K, V> {
                                            _phantom_key: std::marker::PhantomData,
                                            _phantom_value: std::marker::PhantomData,
                                        })
    }
}

#[cfg(test)]
mod view_response_jsonable_tests {

    use DocumentId;
    use serde_json;
    use super::{ViewResponseJsonable, ViewRowJsonable};

    #[test]
    fn deserialize_reduced_ok() {
        let expected = ViewResponseJsonable::<String, i32> {
            total_rows: None,
            offset: None,
            update_seq: None,
            rows: vec![ViewRowJsonable {
                           id: None,
                           key: None,
                           value: 42,
                       }],
        };
        let got = serde_json::from_value({
                      serde_json::builder::ObjectBuilder::new()
                          .insert_array("rows", |x| {
                              x.push_object(|x| {
                                  x.insert("key", serde_json::Value::Null)
                                   .insert("value", 42)
                              })
                          })
                          .unwrap()
                  })
                      .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn deserialize_reduced_ok_with_update_seq() {
        let expected = ViewResponseJsonable::<String, i32> {
            total_rows: None,
            offset: None,
            update_seq: Some(17),
            rows: vec![ViewRowJsonable {
                           id: None,
                           key: None,
                           value: 42,
                       }],
        };
        let got = serde_json::from_value({
                      serde_json::builder::ObjectBuilder::new()
                          .insert("update_seq", 17)
                          .insert_array("rows", |x| {
                              x.push_object(|x| {
                                  x.insert("key", serde_json::Value::Null)
                                   .insert("value", 42)
                              })
                          })
                          .unwrap()
                  })
                      .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn deserialize_unreduced_ok() {
        let expected = ViewResponseJsonable {
            total_rows: Some(10),
            offset: Some(5),
            update_seq: None,
            rows: vec![ViewRowJsonable {
                           id: Some(DocumentId::from("foo")),
                           key: Some(String::from("bar")),
                           value: 42,
                       }],
        };
        let got = serde_json::from_value({
                      serde_json::builder::ObjectBuilder::new()
                          .insert("total_rows", 10)
                          .insert("offset", 5)
                          .insert_array("rows", |x| {
                              x.push_object(|x| {
                                  x.insert("id", "foo")
                                   .insert("key", "bar")
                                   .insert("value", 42)
                              })
                          })
                          .unwrap()
                  })
                      .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn deserialize_unreduced_ok_with_update_seq() {
        let expected = ViewResponseJsonable {
            total_rows: Some(10),
            offset: Some(5),
            update_seq: Some(17),
            rows: vec![ViewRowJsonable {
                           id: Some(DocumentId::from("foo")),
                           key: Some(String::from("bar")),
                           value: 42,
                       }],
        };
        let got = serde_json::from_value({
                      serde_json::builder::ObjectBuilder::new()
                          .insert("total_rows", 10)
                          .insert("offset", 5)
                          .insert("update_seq", 17)
                          .insert_array("rows", |x| {
                              x.push_object(|x| {
                                  x.insert("id", "foo")
                                   .insert("key", "bar")
                                   .insert("value", 42)
                              })
                          })
                          .unwrap()
                  })
                      .unwrap();
        assert_eq!(expected, got);
    }
}

#[derive(Debug, PartialEq)]
pub struct ViewRowJsonable<K: serde::Deserialize, V: serde::Deserialize> {
    key: Option<K>,
    value: V,
    id: Option<DocumentId>,
}

impl<K, V> serde::Deserialize for ViewRowJsonable<K, V>
    where K: serde::Deserialize,
          V: serde::Deserialize
{
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: serde::Deserializer
    {
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

        struct Visitor<K2, V2>
            where K2: serde::Deserialize,
                  V2: serde::Deserialize
        {
            _phantom_key: std::marker::PhantomData<K2>,
            _phantom_value: std::marker::PhantomData<V2>,
        }

        impl<K2, V2> serde::de::Visitor for Visitor<K2, V2>
            where K2: serde::Deserialize,
                  V2: serde::Deserialize
{
            type Value = ViewRowJsonable<K2, V2>;

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
        deserializer.deserialize_struct("ViewRowJsonable",
                                        FIELDS,
                                        Visitor::<K, V> {
                                            _phantom_key: std::marker::PhantomData,
                                            _phantom_value: std::marker::PhantomData,
                                        })
    }
}

#[cfg(test)]
mod view_row_jsonable_tests {

    use DocumentId;
    use serde_json;
    use super::*;

    #[test]
    fn deserialize_ok_reduced() {
        let expected = ViewRowJsonable::<String, i32> {
            id: None,
            key: None,
            value: 42,
        };
        let got = serde_json::from_value({
                      serde_json::builder::ObjectBuilder::new()
                          .insert("key", serde_json::Value::Null)
                          .insert("value", 42)
                          .unwrap()
                  })
                      .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn deserialize_ok_unreduced() {
        let expected = ViewRowJsonable {
            id: Some(DocumentId::from("foo")),
            key: Some(String::from("bar")),
            value: 42,
        };
        let got = serde_json::from_value({
                      serde_json::builder::ObjectBuilder::new()
                          .insert("id", "foo")
                          .insert("key", "bar")
                          .insert("value", 42)
                          .unwrap()
                  })
                      .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn deserialize_nok_missing_value() {
        let json_text = serde_json::to_string(&{
                            serde_json::builder::ObjectBuilder::new()
                                .insert("id", "foo")
                                .insert("key", "bar")
                                .unwrap()
                        })
                            .unwrap();
        let got = serde_json::from_str::<ViewRowJsonable<String, i32>>(&json_text);
        expect_json_error_missing_field!(got, "value");
    }
}

/// Builds a view response.
///
/// A `ViewResponseBuilder` constructs a view response as though the response
/// originated from a CouchDB Server. This allows an application to mock a view
/// response without using a production database.
///
/// # Examples
///
/// One point of inconvenience when using a `ViewResponseBuilder` is that type
/// inference doesn't work when building an unreduced view response, so
/// applications should use the turbofish (`::<Key, Value, _>`) to specify the
/// key and value types, like so:
///
/// ```
/// use chill::testing::ViewResponseBuilder;
///
/// ViewResponseBuilder::<String, i32, _>::new_unreduced(123,       // `total_rows`
///                                                      0,         // `offset`
///                                                      "baseball" // database name
///                                                     )
///     .with_row("Hammerin' Hank", 755, "hank_aaron_doc_id")
///     .with_row("The Bambino", 714, "babe_ruth_doc_id")
///     .unwrap();
/// ```
///
/// Reduced views are easier because type inference _does_ work.
///
/// ```
/// use chill::testing::ViewResponseBuilder;
///
/// // The view response key is inferred as () because reduced views have no
/// // key.
/// ViewResponseBuilder::new_reduced(1469).unwrap();
/// ```
///
#[derive(Debug)]
pub struct ViewResponseBuilder<K: serde::Deserialize, V: serde::Deserialize, M> {
    phantom: std::marker::PhantomData<M>,
    db_name: Option<DatabaseName>,
    target: ViewResponse<K, V>,
}

impl<V: serde::Deserialize> ViewResponseBuilder<(), V, ViewIsReduced> {
    /// Constructs a reduced view response.
    pub fn new_reduced(value: V) -> Self {
        ViewResponseBuilder {
            phantom: std::marker::PhantomData,
            db_name: None,
            target: ViewResponse::Reduced(ReducedView {
                update_seq: None,
                value: value,
            }),
        }
    }

    /// Sets the update sequence number for the view response.
    ///
    /// The **update sequence number** corresponds to the `update_seq` field in
    /// the view response. By default, its value is `None`.
    ///
    pub fn with_update_sequence_number(mut self, update_seq: u64) -> Self {
        self.target.as_reduced_mut().unwrap().update_seq = Some(update_seq);
        self
    }

    /// Returns the builder's view response.
    pub fn unwrap(self) -> ViewResponse<(), V> {
        self.target
    }
}

impl<K: serde::Deserialize, V: serde::Deserialize> ViewResponseBuilder<K, V, ViewIsUnreduced> {
    /// Constructs an unreduced view response.
    ///
    /// Any rows added via the `with_row` method will use the given database
    /// name as part of their document path.
    ///
    pub fn new_unreduced<D: Into<DatabaseName>>(total_rows: u64, offset: u64, db_name: D) -> Self {
        ViewResponseBuilder {
            phantom: std::marker::PhantomData,
            db_name: Some(db_name.into()),
            target: ViewResponse::Unreduced(UnreducedView {
                total_rows: total_rows,
                offset: offset,
                update_seq: None,
                rows: Vec::new(),
            }),
        }
    }

    /// Appends a new row into the view response.
    pub fn with_row<D, IntoK, IntoV>(mut self, key: IntoK, value: IntoV, doc_id: D) -> Self
        where D: Into<DocumentId>,
              IntoK: Into<K>,
              IntoV: Into<V>
    {
        let row = ViewRow {
            key: key.into(),
            value: value.into(),
            doc_path: DocumentPath::from((self.db_name.clone().unwrap(), doc_id.into())),
        };

        self.target.as_unreduced_mut().unwrap().rows.push(row);
        self
    }

    /// Sets the update sequence number for the view response.
    ///
    /// The **update sequence number** corresponds to the `update_seq` field in
    /// the view response. By default, its value is `None`.
    ///
    pub fn with_update_sequence_number(mut self, update_seq: u64) -> Self {
        self.target.as_unreduced_mut().unwrap().update_seq = Some(update_seq);
        self
    }

    /// Returns the builder's view response.
    pub fn unwrap(self) -> ViewResponse<K, V> {
        self.target
    }
}

impl<K: serde::Deserialize, M, V: serde::Deserialize> ViewResponseBuilder<K, V, M> {}

#[cfg(test)]
mod view_response_builder_tests {

    use super::*;
    use IntoDocumentPath;

    #[test]
    fn reduced_required() {
        let expected = ViewResponse::Reduced({
            ReducedView {
                update_seq: None,
                value: 42,
            }
        });
        let got = ViewResponseBuilder::new_reduced(42).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn reduced_with_update_sequence_number() {
        let expected = ViewResponse::Reduced({
            ReducedView {
                update_seq: Some(517),
                value: 42,
            }
        });
        let got = ViewResponseBuilder::new_reduced(42).with_update_sequence_number(517).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn unreduced_required() {
        let expected = ViewResponse::Unreduced({
            UnreducedView::<String, i32> {
                total_rows: 20,
                offset: 10,
                update_seq: None,
                rows: Vec::new(),
            }
        });
        let got = ViewResponseBuilder::new_unreduced(20, 10, "foo").unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn unreduced_with_update_sequence_number() {
        let expected = ViewResponse::Unreduced({
            UnreducedView::<String, i32> {
                total_rows: 20,
                offset: 10,
                update_seq: Some(517),
                rows: Vec::new(),
            }
        });
        let got = ViewResponseBuilder::new_unreduced(20, 10, "foo")
                      .with_update_sequence_number(517)
                      .unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn unreduced_with_row() {
        let expected = ViewResponse::Unreduced({
            UnreducedView::<String, i32> {
                total_rows: 20,
                offset: 10,
                update_seq: None,
                rows: vec![ViewRow {
                               key: String::from("Babe Ruth"),
                               value: 714,
                               doc_path: "/baseball/babe_ruth".into_document_path().unwrap(),
                           },
                           ViewRow {
                               key: String::from("Hank Aaron"),
                               value: 755,
                               doc_path: "/baseball/hank_aaron".into_document_path().unwrap(),
                           }],
            }
        });
        let got = ViewResponseBuilder::new_unreduced(20, 10, "baseball")
                      .with_row("Babe Ruth", 714, "babe_ruth")
                      .with_row("Hank Aaron", 755, "hank_aaron")
                      .unwrap();
        assert_eq!(expected, got);
    }
}
