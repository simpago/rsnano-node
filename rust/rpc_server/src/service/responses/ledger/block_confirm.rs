use rsnano_core::BlockHash;
use rsnano_node::{
    consensus::{ElectionStatus, ElectionStatusType},
    Node,
};
use rsnano_rpc_messages::{BoolDto, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn block_confirm(node: Arc<Node>, hash: BlockHash) -> String {
    let tx = node.ledger.read_txn();
    match &node.ledger.any().get_block(&tx, &hash) {
        Some(block) => {
            if !node.ledger.confirmed().block_exists_or_pruned(&tx, &hash) {
                if !node.confirming_set.exists(&hash) {
                    node.election_schedulers
                        .manual
                        .push(Arc::new(block.clone()), None);
                }
            } else {
                let mut status = ElectionStatus::default();
                status.winner = Some(Arc::new(block.clone()));
                status.election_end = std::time::SystemTime::now();
                status.block_count = 1;
                status.election_status_type = ElectionStatusType::ActiveConfirmationHeight;
                node.active.insert_recently_cemented(status);
            }
            let block_confirm = BoolDto::new("started".to_string(), true);
            to_string_pretty(&block_confirm).unwrap()
        }
        None => to_string_pretty(&ErrorDto::new("Block not found".to_string())).unwrap(),
    }
}
