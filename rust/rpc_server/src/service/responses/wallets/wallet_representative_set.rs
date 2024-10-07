use rsnano_core::{Account, WalletId};
use rsnano_node::{node::Node, wallets::WalletsExt};
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

#[cfg(test)]
mod tests {
    use rsnano_core::{Account, PublicKey, WalletId};
    use rsnano_node::wallets::WalletsExt;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn wallet_representative_set() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet = WalletId::zero();
        node.wallets.create(wallet);

        node.tokio.block_on(async {
            rpc_client
                .wallet_representative_set(wallet, Account::zero(), None)
                .await
                .unwrap()
        });

        assert_eq!(
            node.wallets.get_representative(wallet).unwrap(),
            PublicKey::zero()
        );

        server.abort();
    }

    #[test]
    fn wallet_representative_set_fails_without_enable_control() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node.tokio.block_on(async {
            rpc_client
                .wallet_representative_set(WalletId::zero(), Account::zero(), None)
                .await
        });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"RPC control is disabled\"".to_string())
        );

        server.abort();
    }
}
