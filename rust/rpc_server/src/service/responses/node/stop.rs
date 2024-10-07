use rsnano_node::node::{Node, NodeExt};
use rsnano_rpc_messages::{ErrorDto, SuccessDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn stop(node: Arc<Node>, enable_control: bool) -> String {
    if enable_control {
        node.stop();
        to_string_pretty(&SuccessDto::new()).unwrap()
    } else {
        to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn stop() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        node.tokio
            .block_on(async { rpc_client.stop().await.unwrap() });

        assert!(node.is_stopped());

        server.abort();
    }

    #[test]
    fn stop_fails_with_enable_control_disabled() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node.tokio.block_on(async { rpc_client.stop().await });

        assert!(result.is_err());

        server.abort();
    }
}
