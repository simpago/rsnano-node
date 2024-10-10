use rsnano_core::{BlockDetails, BlockHash, BlockSubType, BlockType};
use rsnano_node::Node;
use rsnano_rpc_messages::{BlockInfoDto, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn block_info(node: Arc<Node>, hash: BlockHash) -> String {
    let txn = node.ledger.read_txn();
    let block = if let Some(block) = node.ledger.get_block(&txn, &hash) {
        block
    } else {
        return to_string_pretty(&ErrorDto::new("Block not found".to_string())).unwrap();
    };

    let account = block.account();
    let amount = node.ledger.any().block_amount(&txn, &hash).unwrap();
    let balance = node.ledger.any().block_balance(&txn, &hash).unwrap();
    let height = block.sideband().unwrap().height;
    let local_timestamp = block.sideband().unwrap().timestamp;
    let successor = block.sideband().unwrap().successor;
    let confirmed = node.ledger.confirmed().block_exists_or_pruned(&txn, &hash);
    let contents = block.json_representation();

    let subtype = match block.block_type() {
        BlockType::State => serde_json::from_str::<BlockSubType>(&BlockDetails::state_subtype(
            &block.sideband().unwrap().details,
        ))
        .unwrap(),
        BlockType::LegacyChange => BlockSubType::Change,
        BlockType::LegacyOpen => BlockSubType::Open,
        BlockType::LegacySend => BlockSubType::Send,
        BlockType::LegacyReceive => BlockSubType::Receive,
        _ => return to_string_pretty(&ErrorDto::new("Block error".to_string())).unwrap(),
    };

    let block_info_dto = BlockInfoDto::new(
        account,
        amount,
        balance,
        height,
        local_timestamp,
        successor,
        confirmed,
        contents,
        subtype,
    );

    to_string_pretty(&block_info_dto).unwrap()
}
