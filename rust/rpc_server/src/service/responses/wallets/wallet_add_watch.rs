use rsnano_core::{Account, WalletId};
use rsnano_node::node::Node;
use rsnano_rpc_messages::{ErrorDto, SuccessDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn wallet_add_watch(
    node: Arc<Node>,
    enable_control: bool,
    wallet: WalletId,
    accounts: Vec<Account>,
) -> String {
    if enable_control {
        match node.wallets.insert_watch(&wallet, &accounts) {
            Ok(_) => to_string_pretty(&SuccessDto::new()).unwrap(),
            Err(e) => to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
        }
    } else {
        to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use rsnano_core::{Account, WalletId};
    use rsnano_ledger::DEV_GENESIS_ACCOUNT;
    use rsnano_node::wallets::WalletsExt;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn wallet_add_watch() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet_id = WalletId::zero();

        node.wallets.create(wallet_id);

        node.tokio.block_on(async {
            rpc_client
                .wallet_add_watch(wallet_id, vec![*DEV_GENESIS_ACCOUNT])
                .await
                .unwrap()
        });

        assert!(node.wallets.exists(&(*DEV_GENESIS_ACCOUNT).into()));

        server.abort();
    }

    #[test]
    fn wallet_add_watch_without_enable_control() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let wallet_id = WalletId::zero();

        node.wallets.create(wallet_id);

        let result = node.tokio.block_on(async {
            rpc_client
                .wallet_add_watch(wallet_id, vec![Account::zero()])
                .await
        });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"RPC control is disabled\"".to_string())
        );

        server.abort();
    }
}
