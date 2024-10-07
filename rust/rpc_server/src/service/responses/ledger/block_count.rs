use rsnano_node::node::Node;
use rsnano_rpc_messages::BlockCountDto;
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn block_count(node: Arc<Node>) -> String {
    let count = node.ledger.block_count();
    let unchecked = node.unchecked.buffer_count() as u64;
    let cemented = node.ledger.cemented_count();
    let block_count = BlockCountDto::new(count, unchecked, cemented);
    to_string_pretty(&block_count).unwrap()
}

#[cfg(test)]
mod tests {
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn block_count() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node
            .tokio
            .block_on(async { rpc_client.block_count().await.unwrap() });

        assert_eq!(result.count, 1);
        assert_eq!(result.cemented, 1);
        assert_eq!(result.unchecked, 0);

        server.abort();
    }
}
