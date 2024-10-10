use rsnano_core::{Account, Amount, Block, BlockEnum, BlockHash, BlockSubType, WalletId};
use rsnano_node::Node;
use rsnano_rpc_messages::{ErrorDto, HistoryEntryDto, WalletHistoryDto};
use rsnano_store_lmdb::Transaction;
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn wallet_history(
    node: Arc<Node>,
    wallet: WalletId,
    modified_since: Option<u64>,
) -> String {
    let accounts = match node.wallets.get_accounts_of_wallet(&wallet) {
        Ok(accounts) => accounts,
        Err(e) => {
            let error_dto = ErrorDto::new(e.to_string());
            return to_string_pretty(&error_dto).unwrap();
        }
    };

    let mut entries: Vec<HistoryEntryDto> = Vec::new();

    let block_transaction = node.store.tx_begin_read();

    for account in accounts {
        if let Some(info) = node.ledger.any().get_account(&block_transaction, &account) {
            let mut hash = info.head;

            while !hash.is_zero() {
                if let Some(block) = node.ledger.get_block(&block_transaction, &hash) {
                    let timestamp = block
                        .sideband()
                        .map(|sideband| sideband.timestamp)
                        .unwrap_or_default();

                    if timestamp >= modified_since.unwrap_or(0) {
                        let entry = process_block(
                            &node,
                            &block_transaction,
                            &block,
                            &account,
                            &hash,
                            timestamp,
                        );

                        if let Some(entry) = entry {
                            entries.push(entry);
                        }

                        hash = block.previous();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
    }

    entries.sort_by(|a, b| b.local_timestamp.cmp(&a.local_timestamp));
    let wallet_history_dto = WalletHistoryDto::new(entries);

    to_string_pretty(&wallet_history_dto).unwrap()
}

fn process_block(
    node: &Arc<Node>,
    transaction: &dyn Transaction,
    block: &BlockEnum,
    block_account: &Account,
    hash: &BlockHash,
    timestamp: u64,
) -> Option<HistoryEntryDto> {
    match block {
        BlockEnum::State(state_block) => {
            let balance = state_block.balance();
            let previous_balance = node
                .ledger
                .any()
                .block_balance(transaction, &state_block.previous())
                .unwrap_or(Amount::zero());

            if balance < previous_balance {
                // Send
                let account: Account = state_block.link().into();
                Some(HistoryEntryDto::new(
                    BlockSubType::Send,
                    account,
                    previous_balance - balance,
                    *block_account,
                    *hash,
                    timestamp,
                ))
            } else if !state_block.link().is_zero() && balance > previous_balance {
                // Receive
                let source_account = node
                    .ledger
                    .any()
                    .block_account(transaction, &state_block.link().into())
                    .unwrap_or_else(|| Account::from(state_block.link()));
                Some(HistoryEntryDto::new(
                    BlockSubType::Receive,
                    source_account,
                    balance - previous_balance,
                    *block_account,
                    *hash,
                    timestamp,
                ))
            } else {
                // Change or Epoch (ignored)
                None
            }
        }
        _ => None, // Ignore legacy blocks
    }
}
