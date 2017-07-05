use std;

#[derive(Debug, Deserialize)]
pub struct NokResponse {
    error: String,
    reason: String,
}

impl NokResponse {
    pub fn error(&self) -> &str {
        &self.error
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl std::fmt::Display for NokResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{}: {}", self.error, self.reason)
    }
}
