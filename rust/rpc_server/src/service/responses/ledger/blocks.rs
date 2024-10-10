use rsnano_core::{BlockHash, JsonBlock};
use rsnano_node::Node;
use rsnano_rpc_messages::BlocksDto;
use serde_json::to_string_pretty;
use std::{collections::HashMap, sync::Arc};

pub async fn blocks(node: Arc<Node>, hashes: Vec<BlockHash>) -> String {
    let mut blocks: HashMap<BlockHash, JsonBlock> = HashMap::new();
    let txn = node.ledger.read_txn();
    for hash in hashes {
        let block = node.ledger.get_block(&txn, &hash).unwrap();
        blocks.insert(hash, block.json_representation());
    }
    to_string_pretty(&BlocksDto::new(blocks)).unwrap()
}
