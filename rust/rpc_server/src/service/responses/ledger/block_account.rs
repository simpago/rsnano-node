use rsnano_core::BlockHash;
use rsnano_node::node::Node;
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

#[cfg(test)]
mod tests {
    use rsnano_core::BlockHash;
    use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH};
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn block_account() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node.tokio.block_on(async {
            rpc_client
                .block_account(DEV_GENESIS_HASH.to_owned())
                .await
                .unwrap()
        });

        assert_eq!(result.value, DEV_GENESIS_ACCOUNT.to_owned());

        server.abort();
    }

    #[test]
    fn block_account_fails_with_block_not_found() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node
            .tokio
            .block_on(async { rpc_client.block_account(BlockHash::zero()).await });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"Block not found\"".to_string())
        );

        server.abort();
    }
}
