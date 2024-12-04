use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RpcError {
    pub error: String,
}

impl RpcError {
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
        }
    }
}
