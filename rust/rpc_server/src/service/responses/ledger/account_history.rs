use rsnano_core::{Account, Amount, Block, BlockEnum, BlockHash, BlockSubType};
use rsnano_node::node::Node;
use rsnano_rpc_messages::{AccountHistoryArgs, AccountHistoryDto, HistoryEntry};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn account_history(node: Arc<Node>, args: AccountHistoryArgs) -> String {
    let transaction = node.store.tx_begin_read();
    let mut history = Vec::new();
    let reverse = args.reverse.unwrap_or(false);
    let mut hash = if reverse {
        node.ledger
            .any()
            .get_account(&transaction, &args.account)
            .unwrap_or_default()
            .open_block
    } else {
        args.head.unwrap_or_else(|| {
            node.ledger
                .any()
                .account_head(&transaction, &args.account)
                .unwrap_or_default()
        })
    };
    let mut count = args.count;
    let mut offset = args.offset.unwrap_or(0);
    let raw = args.raw.unwrap_or(false);
    let account_filter = args.account_filter.clone();

    while let Some(block) = node.ledger.get_block(&transaction, &hash) {
        if offset > 0 {
            offset -= 1;
        } else if count > 0 {
            if let Some(entry) =
                create_history_entry(node.clone(), &block, &hash, raw, &account_filter)
            {
                history.push(entry);
                count -= 1;
            }
        } else {
            break;
        }

        hash = if !reverse {
            block.previous()
        } else {
            let a = node
                .ledger
                .any()
                .block_successor(&transaction, &hash)
                .unwrap_or_default();
            a
        };

        if hash.is_zero() {
            break;
        }
    }

    //if reverse {
    //history.reverse();
    //}

    let next = if !hash.is_zero() { Some(hash) } else { None };

    let previous = if !history.is_empty() {
        Some(if reverse {
            history.last().unwrap().hash
        } else {
            history.first().unwrap().hash
        })
    } else {
        None
    };

    let account_history = AccountHistoryDto {
        account: args.account,
        history,
        previous,
        next,
    };

    to_string_pretty(&account_history).unwrap_or_else(|_| "".to_string())
}

fn create_history_entry(
    node: Arc<Node>,
    block: &BlockEnum,
    hash: &BlockHash,
    raw: bool,
    account_filter: &Option<Vec<Account>>,
) -> Option<HistoryEntry> {
    let transaction = node.store.tx_begin_read();
    let confirmed = node
        .ledger
        .confirmed()
        .block_exists_or_pruned(&transaction, hash);
    let local_timestamp = block.sideband().unwrap().timestamp;
    let height = block.sideband().unwrap().height;

    let (block_type, account, amount) = match block {
        BlockEnum::LegacySend(send_block) => {
            let amount = node
                .ledger
                .any()
                .block_amount(&transaction, hash)
                .unwrap_or_default();
            let destination = *send_block.destination();
            if account_filter
                .as_ref()
                .map_or(false, |filter| !filter.contains(&destination))
            {
                return None;
            }
            (BlockSubType::Send, destination, amount)
        }
        BlockEnum::LegacyReceive(receive_block) => {
            let amount = node
                .ledger
                .any()
                .block_amount(&transaction, hash)
                .unwrap_or_default();
            let source_account = node
                .ledger
                .any()
                .block_account(&transaction, &receive_block.source())
                .unwrap_or_default();
            if account_filter
                .as_ref()
                .map_or(false, |filter| !filter.contains(&source_account))
            {
                return None;
            }
            (BlockSubType::Receive, source_account, amount)
        }
        BlockEnum::LegacyOpen(open_block) => {
            let (amount, source_account) = if open_block.source().as_bytes()
                == node.ledger.constants.genesis_account.as_bytes()
            {
                (
                    node.ledger.constants.genesis_amount,
                    node.ledger.constants.genesis_account,
                )
            } else {
                let amount = node
                    .ledger
                    .any()
                    .block_amount(&transaction, hash)
                    .unwrap_or_default();
                let source_account = node
                    .ledger
                    .any()
                    .block_account(&transaction, &open_block.source())
                    .unwrap_or_default();
                if account_filter
                    .as_ref()
                    .map_or(false, |filter| !filter.contains(&source_account))
                {
                    return None;
                } else {
                    (amount, source_account)
                }
            };
            (BlockSubType::Receive, source_account, amount)
        }
        BlockEnum::LegacyChange(_) => {
            if raw {
                (BlockSubType::Change, Account::default(), Amount::zero())
            } else {
                return None; // Skip change blocks if not raw
            }
        }
        BlockEnum::State(state_block) => {
            if state_block.previous().is_zero() {
                // Open block
                let source_account = node
                    .ledger
                    .any()
                    .block_account(&transaction, &state_block.link().into())
                    .unwrap_or_default();
                if account_filter
                    .as_ref()
                    .map_or(false, |filter| !filter.contains(&source_account))
                {
                    return None;
                }
                (BlockSubType::Receive, source_account, state_block.balance())
            } else {
                let previous_balance = node
                    .ledger
                    .any()
                    .block_balance(&transaction, &state_block.previous())
                    .unwrap_or_default();
                if state_block.balance() < previous_balance {
                    // Send block
                    let destination = state_block.link().into();
                    if account_filter
                        .as_ref()
                        .map_or(false, |filter| !filter.contains(&destination))
                    {
                        return None;
                    }
                    (
                        BlockSubType::Send,
                        destination,
                        previous_balance - state_block.balance(),
                    )
                } else if state_block.link().is_zero() {
                    // Change block
                    if raw {
                        (BlockSubType::Change, Account::default(), Amount::zero())
                    } else {
                        return None; // Skip change blocks if not raw
                    }
                } else {
                    // Receive block
                    let source_account = node
                        .ledger
                        .any()
                        .block_account(&transaction, &state_block.link().into())
                        .unwrap_or_default();
                    if account_filter
                        .as_ref()
                        .map_or(false, |filter| !filter.contains(&source_account))
                    {
                        return None;
                    }
                    (
                        BlockSubType::Receive,
                        source_account,
                        state_block.balance() - previous_balance,
                    )
                }
            }
        }
    };

    Some(HistoryEntry {
        block_type,
        account,
        amount,
        local_timestamp,
        height,
        hash: *hash,
        confirmed,
        work: if raw { Some(block.work().into()) } else { None },
        signature: if raw {
            Some(block.block_signature().clone())
        } else {
            None
        },
    })
}

#[cfg(test)]
mod tests {
    use rsnano_core::{Account, Amount, BlockSubType, PublicKey, WalletId, DEV_GENESIS_KEY};
    use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
    use rsnano_node::wallets::WalletsExt;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn account_history() {
        let mut system = System::new();
        let node = system.make_node();

        // Create and process blocks
        let wallet_id = WalletId::zero();
        node.wallets.create(wallet_id);
        node.wallets
            .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), false)
            .unwrap();

        let change = node
            .wallets
            .change_action2(
                &wallet_id,
                *DEV_GENESIS_ACCOUNT,
                *DEV_GENESIS_PUB_KEY,
                node.work_generate_dev((*DEV_GENESIS_HASH).into()),
                false,
            )
            .unwrap()
            .unwrap();

        let send = node
            .wallets
            .send_action2(
                &wallet_id,
                *DEV_GENESIS_ACCOUNT,
                *DEV_GENESIS_ACCOUNT,
                node.config.receive_minimum,
                node.work_generate_dev(change.hash().into()),
                false,
                None,
            )
            .unwrap();

        let receive = node
            .wallets
            .receive_action2(
                &wallet_id,
                send.hash(),
                *DEV_GENESIS_PUB_KEY,
                node.config.receive_minimum,
                *DEV_GENESIS_ACCOUNT,
                node.work_generate_dev(send.hash().into()),
                false,
            )
            .unwrap()
            .unwrap();

        let usend = node
            .wallets
            .send_action2(
                &wallet_id,
                *DEV_GENESIS_ACCOUNT,
                *DEV_GENESIS_ACCOUNT,
                Amount::nano(1_000),
                node.work_generate_dev(receive.hash().into()),
                false,
                None,
            )
            .unwrap();

        let ureceive = node
            .wallets
            .receive_action2(
                &wallet_id,
                usend.hash(),
                *DEV_GENESIS_PUB_KEY,
                Amount::nano(1_000),
                *DEV_GENESIS_ACCOUNT,
                node.work_generate_dev(usend.hash().into()),
                false,
            )
            .unwrap()
            .unwrap();

        let uchange = node
            .wallets
            .change_action2(
                &wallet_id,
                *DEV_GENESIS_ACCOUNT,
                PublicKey::zero(),
                node.work_generate_dev(ureceive.hash().into()),
                false,
            )
            .unwrap()
            .unwrap();

        // Set up RPC client and server
        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let account_history = node.tokio.block_on(async {
            rpc_client
                .account_history(
                    *DEV_GENESIS_ACCOUNT,
                    100,
                    Some(false),
                    None,
                    None,
                    None,
                    None,
                )
                .await
                .unwrap()
        });

        assert_eq!(account_history.account, *DEV_GENESIS_ACCOUNT);
        assert_eq!(account_history.history.len(), 5);

        // Verify history entries
        let history = account_history.history;
        assert_eq!(history[0].block_type, BlockSubType::Receive);
        assert_eq!(history[0].hash, ureceive.hash());
        assert_eq!(history[0].account, *DEV_GENESIS_ACCOUNT);
        assert_eq!(history[0].amount, Amount::nano(1_000));
        assert_eq!(history[0].height, 6);
        assert!(!history[0].confirmed);

        assert_eq!(history[1].block_type, BlockSubType::Send);
        assert_eq!(history[1].hash, usend.hash());
        assert_eq!(history[1].account, *DEV_GENESIS_ACCOUNT);
        assert_eq!(history[1].amount, Amount::nano(1_000));
        assert_eq!(history[1].height, 5);
        assert!(!history[1].confirmed);

        assert_eq!(history[2].block_type, BlockSubType::Receive);
        assert_eq!(history[2].hash, receive.hash());
        assert_eq!(history[2].account, *DEV_GENESIS_ACCOUNT);
        assert_eq!(history[2].amount, node.config.receive_minimum);
        assert_eq!(history[2].height, 4);
        assert!(!history[2].confirmed);

        assert_eq!(history[3].block_type, BlockSubType::Send);
        assert_eq!(history[3].hash, send.hash());
        assert_eq!(history[3].account, *DEV_GENESIS_ACCOUNT);
        assert_eq!(history[3].amount, node.config.receive_minimum);
        assert_eq!(history[3].height, 3);
        assert!(!history[3].confirmed);

        assert_eq!(history[4].block_type, BlockSubType::Receive);
        assert_eq!(history[4].hash, *DEV_GENESIS_HASH);
        assert_eq!(history[4].account, *DEV_GENESIS_ACCOUNT);
        assert_eq!(history[4].amount, node.ledger.constants.genesis_amount);
        assert_eq!(history[4].height, 1);
        assert!(history[4].confirmed);

        // Test count and reverse
        let account_history_reverse = node.tokio.block_on(async {
            rpc_client
                .account_history(*DEV_GENESIS_ACCOUNT, 1, None, None, None, Some(true), None)
                .await
                .unwrap()
        });

        assert_eq!(account_history_reverse.history.len(), 1);
        assert_eq!(account_history_reverse.history[0].height, 1);
        assert_eq!(account_history_reverse.next, Some(change.hash()));

        // Test filtering
        let account2: Account = node
            .wallets
            .deterministic_insert2(&wallet_id, false)
            .unwrap()
            .into();
        let send2 = node
            .wallets
            .send_action2(
                &wallet_id,
                *DEV_GENESIS_ACCOUNT,
                account2,
                node.config.receive_minimum,
                node.work_generate_dev(uchange.hash().into()),
                false,
                None,
            )
            .unwrap();

        let receive2 = node
            .wallets
            .receive_action2(
                &wallet_id,
                send2.hash(),
                account2.into(),
                node.config.receive_minimum,
                account2.into(),
                node.work_generate_dev(send2.hash().into()),
                false,
            )
            .unwrap()
            .unwrap();

        // Test filter for send state blocks
        let account_history_filtered_send = node.tokio.block_on(async {
            rpc_client
                .account_history(
                    *DEV_GENESIS_ACCOUNT,
                    100,
                    None,
                    None,
                    None,
                    None,
                    Some(vec![account2]),
                )
                .await
                .unwrap()
        });

        assert_eq!(account_history_filtered_send.history.len(), 2);
        assert_eq!(
            account_history_filtered_send.history[0].block_type,
            BlockSubType::Send
        );
        assert_eq!(account_history_filtered_send.history[0].account, account2);

        // Test filter for receive state blocks
        let account_history_filtered_receive = node.tokio.block_on(async {
            rpc_client
                .account_history(
                    account2.into(),
                    100,
                    None,
                    None,
                    None,
                    None,
                    Some(vec![*DEV_GENESIS_ACCOUNT]),
                )
                .await
                .unwrap()
        });

        assert_eq!(account_history_filtered_receive.history.len(), 1);
        assert_eq!(
            account_history_filtered_receive.history[0].block_type,
            BlockSubType::Receive
        );
        assert_eq!(
            account_history_filtered_receive.history[0].account,
            *DEV_GENESIS_ACCOUNT
        );

        server.abort();
    }
}
