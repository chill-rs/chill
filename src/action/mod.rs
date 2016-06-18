pub mod create_database;
pub mod create_document;
pub mod delete_document;
pub mod execute_view;
pub mod read_document;
pub mod update_document;

pub use self::create_database::CreateDatabase;
pub use self::create_document::CreateDocument;
pub use self::delete_document::DeleteDocument;
pub use self::execute_view::ExecuteView;
pub use self::read_document::ReadDocument;
pub use self::update_document::UpdateDocument;

pub mod query_keys {

    use {Error, Revision, serde, transport};

    macro_rules! define_query_key {
        ($key_name:ident, $key_str:expr) => {
            pub struct $key_name;

            impl transport::AsQueryKey for $key_name {
                type Key = &'static str;
                fn as_query_key(&self) -> Self::Key {
                    $key_str
                }
            }
        }
    }

    macro_rules! define_query_value_bool {
        ($key_name:ident) => {
            impl transport::AsQueryValue<$key_name> for bool {
                type Value = &'static str;
                fn as_query_value(&self) -> Self::Value {
                    if *self {
                        "true"
                    } else {
                        "false"
                    }
                }
            }
        }
    }

    macro_rules! define_query_value_simple {
        ($key_name:ident, $value_type:ty) => {
            impl transport::AsQueryValue<$key_name> for $value_type {
                type Value = String;
                fn as_query_value(&self) -> Self::Value {
                    self.to_string()
                }
            }
        }
    }

    define_query_key!(AttachmentsQueryKey, "attachments");
    define_query_value_bool!(AttachmentsQueryKey);

    define_query_key!(DescendingQueryKey, "descending");
    define_query_value_bool!(DescendingQueryKey);

    define_query_key!(EndKeyQueryKey, "endkey");
    impl<T> transport::AsQueryValueFallible<EndKeyQueryKey> for T
        where T: serde::Serialize
    {
        type Value = String;
        fn as_query_value_fallible(&self) -> Result<Self::Value, Error> {
            use serde_json;
            serde_json::to_string(self).map_err(|e| Error::JsonEncode { cause: e })
        }
    }

    define_query_key!(GroupLevelQueryKey, "group_level");
    define_query_value_simple!(GroupLevelQueryKey, u32);

    define_query_key!(GroupQueryKey, "group");
    define_query_value_bool!(GroupQueryKey);

    define_query_key!(IncludeDocsQueryKey, "include_docs");
    define_query_value_bool!(IncludeDocsQueryKey);

    define_query_key!(InclusiveEndQueryKey, "inclusive_end");
    define_query_value_bool!(InclusiveEndQueryKey);

    define_query_key!(LimitQueryKey, "limit");
    define_query_value_simple!(LimitQueryKey, u64);

    define_query_key!(ReduceQueryKey, "reduce");
    define_query_value_bool!(ReduceQueryKey);

    define_query_key!(RevisionQueryKey, "rev");
    impl transport::AsQueryValue<RevisionQueryKey> for Revision {
        type Value = String;
        fn as_query_value(&self) -> Self::Value {
            self.to_string()
        }
    }

    define_query_key!(StartKeyQueryKey, "startkey");
    impl<T> transport::AsQueryValueFallible<StartKeyQueryKey> for T
        where T: serde::Serialize
    {
        type Value = String;
        fn as_query_value_fallible(&self) -> Result<Self::Value, Error> {
            use serde_json;
            serde_json::to_string(self).map_err(|e| Error::JsonEncode { cause: e })
        }
    }
}
