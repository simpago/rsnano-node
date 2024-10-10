use crate::RpcCommand;
use rsnano_core::BlockHash;
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn republish(
        hash: BlockHash,
        sources: Option<u64>,
        destinations: Option<u64>,
        count: Option<u64>,
    ) -> Self {
        Self::Republish(RepublishArgs::new(hash, sources, destinations, count))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct RepublishArgs {
    pub hash: BlockHash,
    pub sources: Option<u64>,
    pub destinations: Option<u64>,
    pub count: Option<u64>,
}

impl RepublishArgs {
    pub fn new(
        hash: BlockHash,
        sources: Option<u64>,
        destinations: Option<u64>,
        count: Option<u64>,
    ) -> Self {
        Self {
            hash,
            sources,
            destinations,
            count,
        }
    }
}
