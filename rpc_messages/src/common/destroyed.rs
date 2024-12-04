use super::primitives::RpcBoolNumber;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct DestroyedResponse {
    pub destroyed: RpcBoolNumber,
}

impl DestroyedResponse {
    pub fn new(destroyed: bool) -> Self {
        Self {
            destroyed: destroyed.into(),
        }
    }
}
