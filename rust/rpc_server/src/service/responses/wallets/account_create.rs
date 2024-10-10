use rsnano_core::WalletId;
use rsnano_node::{wallets::WalletsExt, Node};
use rsnano_rpc_messages::{AccountRpcMessage, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn account_create(
    node: Arc<Node>,
    enable_control: bool,
    wallet: WalletId,
    index: Option<u32>,
    work: Option<bool>,
) -> String {
    if !enable_control {
        return to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap();
    }

    let work = work.unwrap_or(true);

    let result = match index {
        Some(i) => node.wallets.deterministic_insert_at(&wallet, i, work),
        None => node.wallets.deterministic_insert2(&wallet, work),
    };

    match result {
        Ok(account) => to_string_pretty(&AccountRpcMessage::new(
            "account".to_string(),
            account.as_account(),
        ))
        .unwrap(),
        Err(e) => to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
    }
}
