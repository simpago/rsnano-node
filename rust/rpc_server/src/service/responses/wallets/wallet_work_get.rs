use rsnano_core::WalletId;
use rsnano_node::node::Node;
use rsnano_rpc_messages::{AccountsWithWorkDto, ErrorDto};
use serde_json::to_string_pretty;
use std::{collections::HashMap, sync::Arc};

pub async fn wallet_work_get(node: Arc<Node>, enable_control: bool, wallet: WalletId) -> String {
    if !enable_control {
        return to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap();
    }

    let accounts = match node.wallets.get_accounts_of_wallet(&wallet) {
        Ok(accounts) => accounts,
        Err(e) => return to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
    };

    let mut works = HashMap::new();

    for account in accounts {
        match node.wallets.work_get2(&wallet, &account.into()) {
            Ok(work) => {
                works.insert(account, work.into());
            }
            Err(e) => return to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
        }
    }

    to_string_pretty(&AccountsWithWorkDto::new(works)).unwrap()
}

#[cfg(test)]
mod tests {
    use rsnano_core::{PublicKey, RawKey, WalletId, WorkNonce};
    use rsnano_node::wallets::WalletsExt;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn wallet_work_get() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet = WalletId::zero();
        let private_key = RawKey::zero();
        let public_key = PublicKey::try_from(&private_key).unwrap().into();

        node.wallets.create(wallet);

        node.wallets
            .insert_adhoc2(&wallet, &private_key, false)
            .unwrap();

        node.wallets.work_set(&wallet, &public_key, 1).unwrap();

        let result = node
            .tokio
            .block_on(async { rpc_client.wallet_work_get(wallet).await.unwrap() });

        assert_eq!(
            result.works.get(&public_key.into()).unwrap(),
            &WorkNonce::from(1)
        );

        server.abort();
    }

    #[test]
    fn wallet_work_get_fails_without_enable_control() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node
            .tokio
            .block_on(async { rpc_client.wallet_work_get(WalletId::zero()).await });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"RPC control is disabled\"".to_string())
        );

        server.abort();
    }

    #[test]
    fn wallet_work_get_fails_with_wallet_not_found() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node
            .tokio
            .block_on(async { rpc_client.wallet_work_get(WalletId::zero()).await });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"Wallet not found\"".to_string())
        );

        server.abort();
    }
}
