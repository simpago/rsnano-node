use crate::command_handler::RpcCommandHandler;
use rsnano_node::consensus::{ElectionStatus, ElectionStatusType};
use rsnano_rpc_messages::{HashRpcMessage, StartedResponse};

impl RpcCommandHandler {
    pub(crate) fn block_confirm(&self, args: HashRpcMessage) -> anyhow::Result<StartedResponse> {
        let tx = self.node.ledger.read_txn();
        let block = self.load_block_any(&tx, &args.hash)?;
        if !self
            .node
            .ledger
            .confirmed()
            .block_exists_or_pruned(&tx, &args.hash)
        {
            // Start new confirmation for unconfirmed (or not being confirmed) block
            if !self.node.confirming_set.exists(&args.hash) {
                self.node.election_schedulers.manual.push(block, None);
            }
        } else {
            // Add record in confirmation history for confirmed block
            let mut status = ElectionStatus::default();
            status.winner = Some(rsnano_core::SavedOrUnsavedBlock::Saved(block));
            status.election_end = std::time::SystemTime::now();
            status.block_count = 1;
            status.election_status_type = ElectionStatusType::ActiveConfirmationHeight;
            self.node.active.insert_recently_cemented(status);
        }
        Ok(StartedResponse::new(true))
    }
}
