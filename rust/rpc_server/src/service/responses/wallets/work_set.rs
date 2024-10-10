use rsnano_core::{Account, WalletId, WorkNonce};
use rsnano_node::Node;
use rsnano_rpc_messages::{ErrorDto, SuccessDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn work_set(
    node: Arc<Node>,
    enable_control: bool,
    wallet: WalletId,
    account: Account,
    work: WorkNonce,
) -> String {
    if enable_control {
        match node.wallets.work_set(&wallet, &account.into(), work.into()) {
            Ok(_) => to_string_pretty(&SuccessDto::new()).unwrap(),
            Err(e) => to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
        }
    } else {
        to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap()
    }
}
