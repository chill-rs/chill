macro_rules! expect_eq_error_response {
    ($got:expr, $expected_error:expr, $expected_reason:expr) => {
        {
            use ErrorResponse;
            let expected = ErrorResponse::new($expected_error, $expected_reason);
            assert_eq!(expected, $got);
        }
    }
}

macro_rules! expect_error_database_exists {
    ($result:expr, $expected_error:expr, $expected_reason:expr) => {
        match $result {
            Err(Error::DatabaseExists(ref error_response)) => {
                expect_eq_error_response!(*error_response, $expected_error, $expected_reason);
            }
            _ => unexpected_result!($result),
        }
    }
}

macro_rules! expect_error_unauthorized {
    ($result:expr, $expected_error:expr, $expected_reason:expr) => {
        match $result {
            Err(Error::Unauthorized(ref error_response)) => {
                expect_eq_error_response!(*error_response, $expected_error, $expected_reason);
            }
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
