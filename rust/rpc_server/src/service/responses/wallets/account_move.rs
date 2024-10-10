use rsnano_core::{Account, PublicKey, WalletId};
use rsnano_node::Node;
use rsnano_rpc_messages::{BoolDto, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn account_move(
    node: Arc<Node>,
    enable_control: bool,
    target: WalletId,
    source: WalletId,
    accounts: Vec<Account>,
) -> String {
    if enable_control {
        let public_keys: Vec<PublicKey> = accounts.iter().map(|account| account.into()).collect();
        let result = node.wallets.move_accounts(&source, &target, &public_keys);

        match result {
            Ok(()) => to_string_pretty(&BoolDto::new("moved".to_string(), true)).unwrap(),
            Err(e) => to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
        }
    } else {
        to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap()
    }
}
