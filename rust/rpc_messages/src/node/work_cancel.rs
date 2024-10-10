use crate::{BlockHashRpcMessage, RpcCommand};
use rsnano_core::BlockHash;

impl RpcCommand {
    pub fn work_cancel(hash: BlockHash) -> Self {
        Self::WorkCancel(BlockHashRpcMessage::new("hash".to_string(), hash))
    }
}
