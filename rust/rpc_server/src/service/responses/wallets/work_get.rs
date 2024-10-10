use rsnano_core::{Account, WalletId};
use rsnano_node::Node;
use rsnano_rpc_messages::{ErrorDto, WorkDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn work_get(
    node: Arc<Node>,
    enable_control: bool,
    wallet: WalletId,
    account: Account,
) -> String {
    if enable_control {
        match node.wallets.work_get2(&wallet, &account.into()) {
            Ok(work) => to_string_pretty(&WorkDto::new(work.into())).unwrap(),
            Err(e) => to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
        }
    } else {
        to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap()
    }
}
