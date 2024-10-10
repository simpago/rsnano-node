use rsnano_core::{Account, WalletId};
use rsnano_node::{wallets::WalletsExt, Node};
use rsnano_rpc_messages::{BoolDto, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn wallet_representative_set(
    node: Arc<Node>,
    enable_control: bool,
    wallet_id: WalletId,
    representative: Account,
    update_existing_accounts: Option<bool>,
) -> String {
    if enable_control {
        let update_existing = update_existing_accounts.unwrap_or(false);
        match node
            .wallets
            .set_representative(wallet_id, representative.into(), update_existing)
        {
            Ok(_) => to_string_pretty(&BoolDto::new("set".to_string(), true)).unwrap(),
            Err(e) => to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
        }
    } else {
        to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap()
    }
}
