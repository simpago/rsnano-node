use rsnano_core::WalletId;
use rsnano_node::node::Node;
use rsnano_rpc_messages::{ErrorDto, FrontiersDto};
use serde_json::to_string_pretty;
use std::{collections::HashMap, sync::Arc};

pub async fn wallet_frontiers(node: Arc<Node>, wallet: WalletId) -> String {
    let tx = node.ledger.read_txn();
    let mut frontiers = HashMap::new();

    let accounts = match node.wallets.get_accounts_of_wallet(&wallet) {
        Ok(accounts) => accounts,
        Err(e) => return to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
    };

    for account in accounts {
        if let Some(block_hash) = node.ledger.any().account_head(&tx, &account) {
            frontiers.insert(account, block_hash);
        }
    }
    to_string_pretty(&FrontiersDto::new(frontiers)).unwrap()
}

#[cfg(test)]
mod tests {
    use rsnano_core::{WalletId, DEV_GENESIS_KEY};
    use rsnano_ledger::DEV_GENESIS_ACCOUNT;
    use rsnano_node::wallets::WalletsExt;
    use test_helpers::{send_block, setup_rpc_client_and_server, System};

    #[test]
    fn wallet_frontiers() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let wallet = WalletId::zero();

        node.wallets.create(wallet);
        node.wallets
            .insert_adhoc2(&wallet, &DEV_GENESIS_KEY.private_key(), false)
            .unwrap();

        let hash = send_block(node.clone());

        let result = node
            .tokio
            .block_on(async { rpc_client.wallet_frontiers(wallet).await.unwrap() });

        assert_eq!(result.frontiers.get(&*DEV_GENESIS_ACCOUNT).unwrap(), &hash);

        server.abort();
    }
}
