// Panics if the given result is not a serde_json 'missing field' error.
//
// NOTE: There's a error-reporting bug in serde_json that makes this check
// impossible. See here: https://github.com/serde-rs/json/issues/22.
//
// When this bug is resolved, we should match for
// `serde_json::ErrorCode::MissingField`. Until then, we use the
// workaround below.
//
macro_rules! expect_json_error_missing_field {
    ($result:expr, $expected_missing_field_name:expr) => {
        match $result {
            Err(serde_json::Error::SyntaxError(serde_json::ErrorCode::ExpectedSomeValue, ref _line, ref _column)) => (),
            _ => unexpected_result!($result),
        }
    }
}
