use crate::RpcCommand;
use rsnano_core::BlockHash;
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn republish(args: RepublishArgs) -> Self {
        Self::Republish(args)
    }
}

impl From<BlockHash> for RepublishArgs {
    fn from(value: BlockHash) -> Self {
        Self::builder(value).build()
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
    pub fn builder(hash: BlockHash) -> RepublishArgsBuilder {
        RepublishArgsBuilder::new(hash)
    }
}

pub struct RepublishArgsBuilder {
    hash: BlockHash,
    sources: Option<u64>,
    destinations: Option<u64>,
    count: Option<u64>,
}

impl RepublishArgsBuilder {
    pub fn new(hash: BlockHash) -> Self {
        Self {
            hash,
            sources: None,
            destinations: None,
            count: None,
        }
    }

    pub fn with_sources(mut self, sources: u64) -> Self {
        self.sources = Some(sources);
        self
    }

    pub fn with_destinations(mut self, destinations: u64) -> Self {
        self.destinations = Some(destinations);
        self
    }

    pub fn with_count(mut self, count: u64) -> Self {
        self.count = Some(count);
        self
    }

    pub fn build(self) -> RepublishArgs {
        RepublishArgs {
            hash: self.hash,
            sources: self.sources,
            destinations: self.destinations,
            count: self.count,
        }
    }
}
