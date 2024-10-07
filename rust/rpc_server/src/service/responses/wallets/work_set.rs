use rsnano_core::{Account, WalletId, WorkNonce};
use rsnano_node::node::Node;
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

#[cfg(test)]
mod tests {
    use rsnano_core::{Account, WalletId};
    use rsnano_node::wallets::WalletsExt;
    use std::{thread::sleep, time::Duration};
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn work_set() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet_id = WalletId::zero();
        node.wallets.create(wallet_id);

        node.tokio.block_on(async {
            rpc_client
                .work_set(wallet_id, Account::zero(), 1.into())
                .await
                .unwrap()
        });

        sleep(Duration::from_millis(1000));

        assert_ne!(
            node.wallets
                .work_get2(&wallet_id, &Account::zero().into())
                .unwrap(),
            0
        );

        server.abort();
    }

    #[test]
    fn work_set_fails_without_enable_control() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node.tokio.block_on(async {
            rpc_client
                .work_set(WalletId::zero(), Account::zero(), 1.into())
                .await
        });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"RPC control is disabled\"".to_string())
        );

        server.abort();
    }

    #[test]
    fn work_set_fails_with_wallet_not_found() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node.tokio.block_on(async {
            rpc_client
                .work_set(WalletId::zero(), Account::zero(), 1.into())
                .await
        });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"Wallet not found\"".to_string())
        );

        server.abort();
    }
}
