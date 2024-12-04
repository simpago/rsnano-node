use crate::RpcBoolNumber;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ValidResponse {
    pub valid: RpcBoolNumber,
}

impl ValidResponse {
    pub fn new(valid: bool) -> Self {
        Self {
            valid: valid.into(),
        }
    }
}
