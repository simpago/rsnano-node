use rsnano_core::{Amount, WalletId};
use rsnano_node::Node;
use rsnano_rpc_messages::{ErrorDto, WalletInfoDto};
use rsnano_store_lmdb::KeyType;
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn wallet_info(node: Arc<Node>, wallet: WalletId) -> String {
    let block_transaction = node.ledger.read_txn();
    let accounts = match node.wallets.get_accounts_of_wallet(&wallet) {
        Ok(accounts) => accounts,
        Err(e) => return to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
    };

    let mut balance = Amount::zero();
    let mut receivable = Amount::zero();
    let mut count = 0u64;
    let mut block_count = 0u64;
    let mut cemented_block_count = 0u64;
    let mut deterministic_count = 0u64;
    let mut adhoc_count = 0u64;

    for account in accounts {
        if let Some(account_info) = node.ledger.account_info(&block_transaction, &account) {
            block_count += account_info.block_count;
            balance += account_info.balance;
        }

        if let Some(confirmation_info) = node
            .store
            .confirmation_height
            .get(&block_transaction, &account)
        {
            cemented_block_count += confirmation_info.height;
        }

        receivable += node
            .ledger
            .account_receivable(&block_transaction, &account, false);

        match node.wallets.key_type(wallet, &account.into()) {
            KeyType::Deterministic => deterministic_count += 1,
            KeyType::Adhoc => adhoc_count += 1,
            _ => (),
        }

        count += 1;
    }

    let deterministic_index = node.wallets.deterministic_index_get(&wallet).unwrap();

    let account_balance = WalletInfoDto::new(
        balance,
        receivable,
        receivable,
        count,
        adhoc_count,
        deterministic_count,
        deterministic_index,
        block_count,
        cemented_block_count,
    );

    to_string_pretty(&account_balance).unwrap()
}
