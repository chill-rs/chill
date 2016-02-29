use base64;
use serde;

#[derive(Debug, PartialEq)]
pub struct SerializableBase64Blob(pub Vec<u8>);

impl serde::Deserialize for SerializableBase64Blob {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: serde::Deserializer
    {
        struct Visitor;

        impl serde::de::Visitor for Visitor {
            type Value = SerializableBase64Blob;

            fn visit_str<E>(&mut self, value: &str) -> Result<Self::Value, E>
                where E: serde::de::Error
            {
                use std::error::Error;
                let blob = try!(base64::u8de(value.as_bytes())
                                    .map_err(|e| E::invalid_value(e.description())));
                Ok(SerializableBase64Blob(blob))
            }
        }

        deserializer.deserialize(Visitor)
    }
}

#[cfg(test)]
mod tests {

    use serde_json;
    use super::SerializableBase64Blob;

    #[test]
    fn deserialize_ok() {
        let expected = SerializableBase64Blob("hello".to_owned().into_bytes());
        let source = serde_json::Value::String("aGVsbG8=".to_string());
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str(&source).unwrap();
        assert_eq!(expected, got);
    }

    #[test]
    fn deserialize_nok_bad_base64() {
        let source = serde_json::Value::String("% percent signs are invalid in base64 %"
                                                   .to_string());
        let source = serde_json::to_string(&source).unwrap();
        let got = serde_json::from_str::<SerializableBase64Blob>(&source);
        expect_json_error_invalid_value!(got);
    }
}
