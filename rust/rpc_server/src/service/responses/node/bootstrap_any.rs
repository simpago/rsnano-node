use rsnano_core::Account;
use rsnano_node::{bootstrap::BootstrapInitiatorExt, Node};
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
