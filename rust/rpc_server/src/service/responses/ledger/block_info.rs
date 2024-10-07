use rsnano_core::{BlockDetails, BlockHash, BlockSubType, BlockType};
use rsnano_node::node::Node;
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

#[cfg(test)]
mod tests {
    use rsnano_core::{Amount, BlockHash, BlockSubType};
    use rsnano_ledger::{DEV_GENESIS, DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH};
    use std::time::{SystemTime, UNIX_EPOCH};
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn block_info() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node
            .tokio
            .block_on(async { rpc_client.block_info(*DEV_GENESIS_HASH).await.unwrap() });

        assert_eq!(result.amount, Amount::MAX);
        assert_eq!(result.balance, Amount::MAX);
        assert_eq!(result.block_account, *DEV_GENESIS_ACCOUNT);
        assert_eq!(result.confirmed, true);
        assert_eq!(result.height, 1);
        assert_eq!(result.subtype, BlockSubType::Open);
        assert_eq!(result.successor, BlockHash::zero());
        assert_eq!(result.contents, DEV_GENESIS.json_representation());

        let current_unix_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as u64;
        assert!(result.local_timestamp <= current_unix_timestamp);

        server.abort();
    }
}
