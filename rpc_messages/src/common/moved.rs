use super::primitives::RpcBoolNumber;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct MovedResponse {
    pub moved: RpcBoolNumber,
}

impl MovedResponse {
    pub fn new(moved: bool) -> Self {
        Self {
            moved: moved.into(),
        }
    }
}
