use crate::command_handler::RpcCommandHandler;
use anyhow::bail;
use rsnano_core::BlockHash;
use rsnano_node::Node;
use rsnano_rpc_messages::{ExistsResponse, ReceivableExistsArgs};
use rsnano_store_lmdb::LmdbReadTransaction;
use std::sync::Arc;

impl RpcCommandHandler {
    pub(crate) fn receivable_exists(
        &self,
        args: ReceivableExistsArgs,
    ) -> anyhow::Result<ExistsResponse> {
        let include_active = args.include_active.unwrap_or_default().inner();
        let include_only_confirmed = args.include_only_confirmed.unwrap_or(true.into()).inner();
        let txn = self.node.ledger.read_txn();

        let Some(block) = self.node.ledger.any().get_block(&txn, &args.hash) else {
            bail!(Self::BLOCK_NOT_FOUND);
        };

        let mut exists = if block.is_send() {
            let pending_key = rsnano_core::PendingKey::new(block.destination().unwrap(), args.hash);
            self.node
                .ledger
                .any()
                .get_pending(&txn, &pending_key)
                .is_some()
        } else {
            false
        };

        if exists {
            exists = block_confirmed(
                self.node.clone(),
                &txn,
                &args.hash,
                include_active,
                include_only_confirmed,
            );
        }
        Ok(ExistsResponse::new(exists))
    }
}

/** Due to the asynchronous nature of updating confirmation heights, it can also be necessary to check active roots */
fn block_confirmed(
    node: Arc<Node>,
    txn: &LmdbReadTransaction,
    hash: &BlockHash,
    include_active: bool,
    include_only_confirmed: bool,
) -> bool {
    if include_active && !include_only_confirmed {
        return true;
    }

    // Check whether the confirmation height is set
    if node.ledger.confirmed().block_exists_or_pruned(txn, hash) {
        return true;
    }

    // This just checks it's not currently undergoing an active transaction
    if !include_only_confirmed {
        if let Some(block) = node.ledger.any().get_block(txn, hash) {
            return !node.active.active(&block);
        }
    }

    false
}
