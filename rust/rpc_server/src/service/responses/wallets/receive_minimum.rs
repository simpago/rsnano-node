use rsnano_node::node::Node;
use rsnano_rpc_messages::{AmountDto, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn receive_minimum(node: Arc<Node>, enable_control: bool) -> String {
    if enable_control {
        let amount = *node.config.receive_minimum.lock().unwrap();
        to_string_pretty(&AmountDto::new("amount".to_string(), amount)).unwrap()
    } else {
        to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn receive_minimum() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node
            .tokio
            .block_on(async { rpc_client.receive_minimum().await.unwrap() });

        assert_eq!(result.value, node.config.receive_minimum);

        server.abort();
    }

    #[test]
    fn receive_minimum_fails_without_enable_control() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node
            .tokio
            .block_on(async { rpc_client.receive_minimum().await });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"RPC control is disabled\"".to_string())
        );

        server.abort();
    }
}
