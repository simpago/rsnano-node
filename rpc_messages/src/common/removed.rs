use crate::RpcBoolNumber;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct RemovedDto {
    pub removed: RpcBoolNumber,
}

impl RemovedDto {
    pub fn new(removed: bool) -> Self {
        Self {
            removed: removed.into(),
        }
    }
}
