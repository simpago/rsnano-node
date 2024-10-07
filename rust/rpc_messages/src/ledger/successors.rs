use rsnano_core::BlockHash;
use crate::RpcCommand;
use super::ChainArgs;

impl RpcCommand {
    pub fn chain(block: BlockHash, count: u64, offset: Option<u64>, reverse: Option<bool>) -> Self {
        Self::Successors(ChainArgs::new(block, count, offset, reverse))
    }
}