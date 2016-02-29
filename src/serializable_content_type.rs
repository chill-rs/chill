use mime;
use serde;

#[derive(Debug, PartialEq)]
pub struct SerializableContentType(pub mime::Mime);

impl serde::Deserialize for SerializableContentType {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: serde::Deserializer
    {
        struct Visitor;

        impl serde::de::Visitor for Visitor {
            type Value = SerializableContentType;

            fn visit_str<E>(&mut self, v: &str) -> Result<Self::Value, E>
                where E: serde::de::Error
            {
                let m = try!(v.parse().map_err(|_| E::invalid_value("Bad MIME string")));
                Ok(SerializableContentType(m))
            }
        }

        deserializer.deserialize(Visitor)
    }
}

#[cfg(test)]
mod tests {

    use serde_json;
    use super::SerializableContentType;

    #[test]
    fn deserialize_ok() {
        let expected = SerializableContentType(mime!(Application / Json));
        let source = serde_json::Value::String("application/json".to_string());
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str(&source).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn deserialize_nok_bad_mime() {
        let source = serde_json::Value::String("bad mime".to_string());
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<SerializableContentType>(&source);
        expect_json_error_invalid_value!(got);
    }
}
