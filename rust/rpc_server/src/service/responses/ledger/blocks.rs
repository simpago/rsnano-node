use rsnano_core::{BlockHash, JsonBlock};
use rsnano_node::node::Node;
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

#[cfg(test)]
mod tests {
    use rsnano_ledger::{DEV_GENESIS, DEV_GENESIS_HASH};
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn blocks() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node
            .tokio
            .block_on(async { rpc_client.blocks(vec![*DEV_GENESIS_HASH]).await.unwrap() });

        assert_eq!(
            result.blocks.get(&DEV_GENESIS_HASH).unwrap(),
            &DEV_GENESIS.json_representation()
        );

        server.abort();
    }
}
