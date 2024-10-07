use rsnano_node::node::Node;
use rsnano_rpc_messages::U64RpcMessage;
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn frontier_count(node: Arc<Node>) -> String {
    to_string_pretty(&U64RpcMessage::new(
        "count".to_string(),
        node.ledger.account_count(),
    ))
    .unwrap()
}

#[cfg(test)]
mod tests {
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn frontier_count() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node
            .tokio
            .block_on(async { rpc_client.frontier_count().await.unwrap() });

        assert_eq!(result.value, 1);

        server.abort();
    }
}
