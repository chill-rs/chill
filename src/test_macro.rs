// FIXME: Remove unused macros.

// macro_rules! expect_eq_error_response {
//     ($got:expr, $expected_error:expr, $expected_reason:expr) => {
//         {
//             use ErrorResponse;
//             let expected = ErrorResponse::new($expected_error, $expected_reason);
//             assert_eq!(expected, $got);
//         }
//     }
// }

// macro_rules! expect_error_database_exists {
//     ($result:expr, $expected_error:expr, $expected_reason:expr) => {
//         {
//             use Error;
//             match $result {
//                 Err(Error::DatabaseExists(ref error_response)) => {
//                     expect_eq_error_response!(*error_response, $expected_error, $expected_reason);
//                 }
//                 _ => unexpected_result!($result),
//             }
//         }
//     }
// }

// macro_rules! expect_error_document_conflict {
//     ($result:expr, $expected_error:expr, $expected_reason:expr) => {
//         {
//             use Error;
//             match $result {
//                 Err(Error::DocumentConflict(ref error_response)) => {
//                     expect_eq_error_response!(*error_response, $expected_error, $expected_reason);
//                 }
//                 _ => unexpected_result!($result),
//             }
//         }
//     }
// }

// macro_rules! expect_error_mock {
//     ($result:expr) => {
//         {
//             use Error;
//             match $result {
//                 Err(Error::Mock { .. }) => (),
//                 _ => unexpected_result!($result),
//             }
//         }
//     }
// }

// macro_rules! expect_error_unauthorized {
//     ($result:expr, $expected_error:expr, $expected_reason:expr) => {
//         {
//             use Error;
//             match $result {
//                 Err(Error::Unauthorized(ref error_response)) => {
//                     expect_eq_error_response!(*error_response, $expected_error, $expected_reason);
//                 }
//                 _ => unexpected_result!($result),
//             }
//         }
//     }
// }

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
