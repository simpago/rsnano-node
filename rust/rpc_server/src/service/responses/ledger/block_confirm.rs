use rsnano_core::BlockHash;
use rsnano_node::{
    consensus::{ElectionStatus, ElectionStatusType},
    node::Node,
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

#[cfg(test)]
mod tests {
    use rsnano_core::BlockHash;
    use rsnano_ledger::DEV_GENESIS_HASH;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn block_confirm() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node.tokio.block_on(async {
            rpc_client
                .block_confirm(DEV_GENESIS_HASH.to_owned())
                .await
                .unwrap()
        });

        assert_eq!(result.value, true);

        server.abort();
    }

    #[test]
    fn block_confirm_fails_with_block_not_found() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node
            .tokio
            .block_on(async { rpc_client.block_confirm(BlockHash::zero()).await });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"Block not found\"".to_string())
        );

        server.abort();
    }
}
