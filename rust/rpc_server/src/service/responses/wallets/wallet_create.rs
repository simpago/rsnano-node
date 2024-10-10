use rsnano_core::{RawKey, WalletId};
use rsnano_node::{wallets::WalletsExt, Node};
use rsnano_rpc_messages::{ErrorDto, WalletCreateDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn wallet_create(node: Arc<Node>, enable_control: bool, seed: Option<RawKey>) -> String {
    if !enable_control {
        return to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap();
    }

    let wallet = WalletId::random();
    node.wallets.create(wallet);
    let mut wallet_create_dto = WalletCreateDto::new(wallet);

    if let Some(seed) = seed {
        let (restored_count, first_account) = node
            .wallets
            .change_seed(wallet, &seed, 0)
            .expect("This should not fail since the wallet was just created");
        wallet_create_dto.last_restored_account = Some(first_account);
        wallet_create_dto.restored_count = Some(restored_count);
    }

    to_string_pretty(&wallet_create_dto).unwrap()
}
