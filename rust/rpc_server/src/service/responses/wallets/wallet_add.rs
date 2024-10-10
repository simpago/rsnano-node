use rsnano_core::{RawKey, WalletId};
use rsnano_node::{wallets::WalletsExt, Node};
use rsnano_rpc_messages::{AccountRpcMessage, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn wallet_add(
    node: Arc<Node>,
    enable_control: bool,
    wallet: WalletId,
    raw_key: RawKey,
    work: Option<bool>,
) -> String {
    if enable_control {
        let generate_work = work.unwrap_or(false);
        match node.wallets.insert_adhoc2(&wallet, &raw_key, generate_work) {
            Ok(account) => to_string_pretty(&AccountRpcMessage::new(
                "account".to_string(),
                account.as_account(),
            ))
            .unwrap(),
            Err(e) => to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
        }
    } else {
        to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap()
    }
}
