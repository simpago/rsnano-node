use rsnano_core::Account;
use rsnano_node::{wallets::WalletsExt, Node};
use rsnano_rpc_messages::{AccountsCreateArgs, AccountsRpcMessage, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn accounts_create(
    node: Arc<Node>,
    enable_control: bool,
    args: AccountsCreateArgs,
) -> String {
    if !enable_control {
        return to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap();
    }

    let work = args.work.unwrap_or(false);
    let count = args.wallet_with_count.count as usize;
    let wallet = &args.wallet_with_count.wallet;

    let accounts: Result<Vec<Account>, _> = (0..count)
        .map(|_| node.wallets.deterministic_insert2(wallet, work))
        .map(|result| result.map(|public_key| public_key.into()))
        .collect();

    match accounts {
        Ok(accounts) => to_string_pretty(&AccountsRpcMessage::new(accounts)).unwrap(),
        Err(e) => to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
    }
}
