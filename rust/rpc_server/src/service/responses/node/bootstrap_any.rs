use rsnano_core::Account;
use rsnano_node::{bootstrap::BootstrapInitiatorExt, node::Node};
use rsnano_rpc_messages::{ErrorDto, SuccessDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn bootstrap_any(
    node: Arc<Node>,
    force: Option<bool>,
    id: Option<String>,
    account: Option<Account>,
) -> String {
    if node.flags.disable_legacy_bootstrap {
        return to_string_pretty(&ErrorDto::new("Bootstrap legacy is disabled".to_string()))
            .unwrap();
    }

    let force = force.unwrap_or(false);
    let bootstrap_id = id.unwrap_or_default();
    let start_account = account.unwrap_or_default();

    node.bootstrap_initiator
        .bootstrap(force, bootstrap_id, u32::MAX, start_account);

    to_string_pretty(&SuccessDto::new()).unwrap()
}

#[cfg(test)]
mod tests {
    use rsnano_node::config::NodeFlags;
    use test_helpers::{send_block, setup_rpc_client_and_server, System};

    #[test]
    fn bootstrap_any() {
        let mut system = System::new();
        let node = system.make_node();

        send_block(node.clone());

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        node.tokio
            .block_on(async { rpc_client.bootstrap_any(None, None, None).await.unwrap() });

        server.abort();
    }

    #[test]
    fn bootstrap_any_fails_with_legacy_bootstrap_disabled() {
        let mut system = System::new();
        let mut flags = NodeFlags::new();
        flags.disable_legacy_bootstrap = true;
        let node = system.build_node().flags(flags).finish();

        send_block(node.clone());

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node
            .tokio
            .block_on(async { rpc_client.bootstrap_any(None, None, None).await });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"Bootstrap legacy is disabled\"".to_string())
        );

        server.abort();
    }
}
