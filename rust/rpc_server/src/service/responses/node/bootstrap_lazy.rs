use rsnano_core::BlockHash;
use rsnano_node::{bootstrap::BootstrapInitiatorExt, node::Node};
use rsnano_rpc_messages::{BootstrapLazyDto, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn bootstrap_lazy(
    node: Arc<Node>,
    hash: BlockHash,
    force: Option<bool>,
    id: Option<String>,
) -> String {
    if node.flags.disable_lazy_bootstrap {
        return to_string_pretty(&ErrorDto::new("Lazy bootstrap is disabled".to_string())).unwrap();
    }

    let force = force.unwrap_or(false);

    let existed = node.bootstrap_initiator.current_lazy_attempt();

    let bootstrap_id = id.unwrap_or_default();

    let key_inserted = node
        .bootstrap_initiator
        .bootstrap_lazy(hash.into(), force, bootstrap_id);

    let started = !existed.is_some() && key_inserted;

    to_string_pretty(&BootstrapLazyDto::new(started, key_inserted)).unwrap()
}

#[cfg(test)]
mod tests {
    use rsnano_core::BlockHash;
    use rsnano_node::config::NodeFlags;
    use test_helpers::{send_block, setup_rpc_client_and_server, System};

    #[test]
    fn bootstrap_any() {
        let mut system = System::new();
        let node = system.make_node();

        let hash = send_block(node.clone());

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node
            .tokio
            .block_on(async { rpc_client.bootstrap_lazy(hash, None, None).await.unwrap() });

        assert_eq!(result.started, true);
        assert_eq!(result.key_inserted, true);

        server.abort();
    }

    #[test]
    fn bootstrap_any_fails_with_legacy_bootstrap_disabled() {
        let mut system = System::new();
        let mut flags = NodeFlags::new();
        flags.disable_lazy_bootstrap = true;
        let node = system.build_node().flags(flags).finish();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node.tokio.block_on(async {
            rpc_client
                .bootstrap_lazy(BlockHash::zero(), None, None)
                .await
        });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"Lazy bootstrap is disabled\"".to_string())
        );

        server.abort();
    }
}
