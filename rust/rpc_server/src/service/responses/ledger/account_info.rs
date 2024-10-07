use rsnano_core::{Account, Amount};
use rsnano_node::node::Node;
use rsnano_rpc_messages::{AccountInfoArgs, AccountInfoDto, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn account_info(node: Arc<Node>, args: AccountInfoArgs) -> String {
    let txn = node.ledger.read_txn();
    let include_confirmed = args.include_confirmed.unwrap_or(false);

    let info = match node.ledger.any().get_account(&txn, &args.account) {
        Some(account_info) => account_info,
        None => return to_string_pretty(&ErrorDto::new("Account not found".to_string())).unwrap(),
    };

    let confirmation_height_info = node
        .store
        .confirmation_height
        .get(&txn, &args.account)
        .unwrap();

    let mut account_info = AccountInfoDto::new(
        info.head,
        info.open_block,
        node.ledger.representative_block_hash(&txn, &info.head),
        info.balance,
        info.modified,
        info.block_count,
        info.epoch as u8,
    );

    account_info.confirmed_height = Some(confirmation_height_info.height);
    account_info.confirmation_height_frontier = Some(confirmation_height_info.frontier);

    if include_confirmed {
        let confirmed_balance = if info.block_count != confirmation_height_info.height {
            node.ledger
                .any()
                .block_balance(&txn, &confirmation_height_info.frontier)
                .unwrap_or(Amount::zero())
        } else {
            info.balance
        };
        account_info.confirmed_balance = Some(confirmed_balance);
    }

    if args.representative.unwrap_or(false) {
        account_info.representative = Some(info.representative.into());
        if include_confirmed {
            let confirmed_representative = if confirmation_height_info.height > 0 {
                if let Some(confirmed_frontier_block) = node
                    .ledger
                    .any()
                    .get_block(&txn, &confirmation_height_info.frontier)
                {
                    confirmed_frontier_block
                        .representative_field()
                        .unwrap_or_else(|| {
                            let rep_block_hash = node.ledger.representative_block_hash(
                                &txn,
                                &confirmation_height_info.frontier,
                            );
                            node.ledger
                                .any()
                                .get_block(&txn, &rep_block_hash)
                                .unwrap()
                                .representative_field()
                                .unwrap()
                        })
                } else {
                    info.representative
                }
            } else {
                info.representative
            };
            account_info.confirmed_representative = Some(confirmed_representative.into());
        }
    }

    if args.weight.unwrap_or(false) {
        account_info.weight = Some(node.ledger.weight_exact(&txn, args.account.into()));
    }

    if args.pending.unwrap_or(false) || args.receivable.unwrap_or(false) {
        let account_receivable = node.ledger.account_receivable(&txn, &args.account, false);
        account_info.pending = Some(account_receivable);
        account_info.receivable = Some(account_receivable);

        if include_confirmed {
            let confirmed_receivable = node.ledger.account_receivable(&txn, &args.account, true);
            account_info.confirmed_pending = Some(confirmed_receivable);
            account_info.confirmed_receivable = Some(confirmed_receivable);
        }
    }

    to_string_pretty(&account_info).unwrap()
}

#[cfg(test)]
mod tests {
    use rsnano_core::{Account, Amount, BlockHash, Epoch};
    use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH};
    use rsnano_rpc_messages::AccountInfoArgs;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn account_info() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node.tokio.block_on(async {
            rpc_client
                .account_info(
                    Account::decode_account(
                        "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
                    )
                    .unwrap(),
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await
        });

        assert_eq!(
            result.err().map(|e| e.to_string()),
            Some("node returned error: \"Account not found\"".to_string())
        );

        let result = node.tokio.block_on(async {
            rpc_client
                .account_info(
                    *DEV_GENESIS_ACCOUNT,
                    Some(true),
                    Some(true),
                    Some(true),
                    Some(true),
                    Some(true),
                )
                .await
                .unwrap()
        });

        assert_eq!(result.frontier, *DEV_GENESIS_HASH);
        assert_eq!(result.open_block, *DEV_GENESIS_HASH);
        assert_eq!(result.representative_block, *DEV_GENESIS_HASH);
        assert_eq!(result.balance, Amount::MAX);
        assert!(result.modified_timestamp > 0);
        assert_eq!(result.block_count, 1);
        assert_eq!(result.account_version, 2);
        assert_eq!(result.confirmed_height, Some(1));
        assert_eq!(result.confirmation_height_frontier, Some(*DEV_GENESIS_HASH));
        assert_eq!(result.representative, Some(*DEV_GENESIS_ACCOUNT));
        assert_eq!(result.weight, Some(Amount::MAX));
        assert_eq!(result.pending, Some(Amount::raw(0)));
        assert_eq!(result.receivable, Some(Amount::raw(0)));
        assert_eq!(result.confirmed_balance, Some(Amount::MAX));
        assert_eq!(result.confirmed_pending, Some(Amount::raw(0)));
        assert_eq!(result.confirmed_receivable, Some(Amount::raw(0)));
        assert_eq!(result.confirmed_representative, Some(*DEV_GENESIS_ACCOUNT));

        server.abort();
    }
}
