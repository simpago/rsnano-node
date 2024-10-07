use rsnano_node::node::Node;
use rsnano_rpc_messages::SuccessDto;
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn unchecked_clear(node: Arc<Node>) -> String {
    node.unchecked.clear();
    to_string_pretty(&SuccessDto::new()).unwrap()
}

#[cfg(test)]
mod tests {
    use test_helpers::{send_block, setup_rpc_client_and_server, System};

    #[test]
    fn unchecked_clear() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        send_block(node.clone());

        assert!(!node.unchecked.is_empty());

        node.tokio
            .block_on(async { rpc_client.unchecked_clear().await.unwrap() });

        assert!(node.unchecked.is_empty());

        server.abort();
    }
}
