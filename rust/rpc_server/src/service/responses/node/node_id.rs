use rsnano_node::node::Node;
use rsnano_rpc_messages::{ErrorDto, NodeIdDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn node_id(node: Arc<Node>, enable_control: bool) -> String {
    if enable_control {
        let private = node.node_id.private_key();
        let public = node.node_id.public_key();
        let as_account = public.as_account();

        to_string_pretty(&NodeIdDto::new(private, public, as_account)).unwrap()
    } else {
        to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn node_id() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        node.tokio
            .block_on(async { rpc_client.node_id().await.unwrap() });

        server.abort();
    }

    #[test]
    fn node_id_without_enable_control() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node.tokio.block_on(async { rpc_client.node_id().await });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"RPC control is disabled\"".to_string())
        );

        server.abort();
    }
}
