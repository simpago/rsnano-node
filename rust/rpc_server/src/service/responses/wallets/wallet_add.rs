use rsnano_core::{RawKey, WalletId};
use rsnano_node::{node::Node, wallets::WalletsExt};
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

#[cfg(test)]
mod tests {
    use rsnano_core::{PublicKey, RawKey, WalletId};
    use rsnano_node::wallets::WalletsExt;
    use std::{thread::sleep, time::Duration};
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn account_create_index_none() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet_id = WalletId::random();

        node.wallets.create(wallet_id);

        let private_key = RawKey::random();
        let public_key: PublicKey = (&private_key).try_into().unwrap();

        node.tokio.block_on(async {
            rpc_client
                .wallet_add(wallet_id, private_key, None)
                .await
                .unwrap()
        });

        assert!(node
            .wallets
            .get_accounts_of_wallet(&wallet_id)
            .unwrap()
            .contains(&public_key.into()));

        server.abort();
    }

    #[test]
    fn account_create_fails_without_enable_control() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let wallet_id = WalletId::random();

        node.wallets.create(wallet_id);

        let private_key = RawKey::random();

        let result = node
            .tokio
            .block_on(async { rpc_client.wallet_add(wallet_id, private_key, None).await });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"RPC control is disabled\"".to_string())
        );

        server.abort();
    }

    #[test]
    fn wallet_add_fails_with_wallet_not_found() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node.tokio.block_on(async {
            rpc_client
                .wallet_add(WalletId::zero(), RawKey::zero(), None)
                .await
        });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"Wallet not found\"".to_string())
        );

        server.abort();
    }

    #[test]
    fn wallet_add_work_true() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet_id = WalletId::random();

        node.wallets.create(wallet_id);

        let private_key = RawKey::random();

        let result = node.tokio.block_on(async {
            rpc_client
                .wallet_add(wallet_id, private_key, Some(true))
                .await
                .unwrap()
        });

        sleep(Duration::from_millis(2000));

        assert_ne!(
            node.wallets
                .work_get2(&wallet_id, &result.value.into())
                .unwrap(),
            0
        );

        server.abort();
    }

    #[test]
    fn wallet_add_work_false() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet_id = WalletId::random();

        node.wallets.create(wallet_id);

        let private_key = RawKey::random();

        let result = node.tokio.block_on(async {
            rpc_client
                .wallet_add(wallet_id, private_key, Some(false))
                .await
                .unwrap()
        });

        sleep(Duration::from_millis(2000));

        assert_eq!(
            node.wallets
                .work_get2(&wallet_id, &result.value.into())
                .unwrap(),
            0
        );

        server.abort();
    }
}
