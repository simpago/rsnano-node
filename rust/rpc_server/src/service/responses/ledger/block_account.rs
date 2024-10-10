use rsnano_core::BlockHash;
use rsnano_node::Node;
use rsnano_rpc_messages::{AccountRpcMessage, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn block_account(node: Arc<Node>, hash: BlockHash) -> String {
    let tx = node.ledger.read_txn();
    match &node.ledger.any().get_block(&tx, &hash) {
        Some(block) => {
            let account = block.account();
            let block_account = AccountRpcMessage::new("account".to_string(), account);
            to_string_pretty(&block_account).unwrap()
        }
        None => to_string_pretty(&ErrorDto::new("Block not found".to_string())).unwrap(),
    }
}
