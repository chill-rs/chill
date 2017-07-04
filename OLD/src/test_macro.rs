// Panics if the given result is not a serde_json 'invalid value' error.
macro_rules! expect_json_error_invalid_value {
    ($result:ident) => {
        match $result {
            Err(serde_json::Error::Syntax(serde_json::ErrorCode::InvalidValue(..), ref _line, ref _column)) => (),
            _ => unexpected_result!($result),
        }
    }
}

// Panics if the given result is not a serde_json 'missing field' error.
macro_rules! expect_json_error_missing_field {
    ($result:expr, $expected_missing_field_name:expr) => {
        match $result {
            Err(serde_json::Error::Syntax(serde_json::ErrorCode::MissingField(field_name), ref _line, ref _column))
                if field_name == $expected_missing_field_name => (),
            _ => unexpected_result!($result),
        }
    }
}

macro_rules! unexpected_result {
    ($result:expr) => {
        match $result {
            Err(e) => panic!("Got unexpected error result {:?}", e),
            Ok(x) => panic!("Got unexpected OK result {:?}", x),
        }
    }
}
