use crate::{RpcCommand, RpcUsize};
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
    pub sources: Option<RpcUsize>,
    pub destinations: Option<RpcUsize>,
    pub count: Option<RpcUsize>,
}

impl RepublishArgs {
    pub fn builder(hash: BlockHash) -> RepublishArgsBuilder {
        RepublishArgsBuilder::new(hash)
    }
}

pub struct RepublishArgsBuilder {
    hash: BlockHash,
    sources: Option<RpcUsize>,
    destinations: Option<RpcUsize>,
    count: Option<RpcUsize>,
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

    pub fn with_sources(mut self, sources: usize) -> Self {
        self.sources = Some(sources.into());
        self
    }

    pub fn with_destinations(mut self, destinations: usize) -> Self {
        self.destinations = Some(destinations.into());
        self
    }

    pub fn with_count(mut self, count: usize) -> Self {
        self.count = Some(count.into());
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
