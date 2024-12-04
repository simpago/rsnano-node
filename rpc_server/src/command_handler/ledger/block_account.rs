use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::{AccountResponse, HashRpcMessage};

impl RpcCommandHandler {
    pub(crate) fn block_account(&self, args: HashRpcMessage) -> anyhow::Result<AccountResponse> {
        let tx = self.node.ledger.read_txn();
        let block = self.load_block_any(&tx, &args.hash)?;
        Ok(AccountResponse::new(block.account()))
    }
}
