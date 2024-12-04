use super::primitives::RpcBoolNumber;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ExistsResponse {
    pub exists: RpcBoolNumber,
}

impl ExistsResponse {
    pub fn new(exists: bool) -> Self {
        Self {
            exists: exists.into(),
        }
    }
}
