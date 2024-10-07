use rsnano_core::BlockHash;
use crate::{BlockHashRpcMessage, RpcCommand};

impl RpcCommand {
    pub fn work_cancel(hash: BlockHash) -> Self {
        Self::WorkCancel(BlockHashRpcMessage::new("hash".to_string(), hash))
    }
}

