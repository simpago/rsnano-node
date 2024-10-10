use rsnano_node::{wallets::WalletsExt, Node};
use rsnano_rpc_messages::{ErrorDto, WalletChangeSeedArgs, WalletChangeSeedDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn wallet_change_seed(
    node: Arc<Node>,
    enable_control: bool,
    args: WalletChangeSeedArgs,
) -> String {
    if enable_control {
        let (restored_count, last_restored_account) = node
            .wallets
            .change_seed(args.wallet, &args.seed, args.count.unwrap_or(0))
            .unwrap();
        to_string_pretty(&WalletChangeSeedDto::new(
            last_restored_account,
            restored_count,
        ))
        .unwrap()
    } else {
        to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap()
    }
}
