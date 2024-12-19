use crate::command_handler::RpcCommandHandler;
use rsnano_core::{Amount, Epoch};
use rsnano_rpc_messages::{unwrap_bool_or_false, AccountInfoArgs, AccountInfoResponse};

impl RpcCommandHandler {
    pub(crate) fn account_info(
        &self,
        args: AccountInfoArgs,
    ) -> anyhow::Result<AccountInfoResponse> {
        let txn = self.node.ledger.read_txn();
        let include_confirmed = unwrap_bool_or_false(args.include_confirmed);
        let info = self.load_account(&txn, &args.account)?;

        let confirmation_height_info = self
            .node
            .store
            .confirmation_height
            .get(&txn, &args.account)
            .unwrap();

        let mut account_info = AccountInfoResponse {
            frontier: info.head,
            open_block: info.open_block,
            representative_block: self.node.ledger.representative_block_hash(&txn, &info.head),
            balance: info.balance,
            modified_timestamp: info.modified.as_u64().into(),
            block_count: info.block_count.into(),
            account_version: (epoch_as_number(info.epoch) as u16).into(),
            confirmed_height: None,
            confirmation_height_frontier: None,
            representative: None,
            weight: None,
            pending: None,
            receivable: None,
            confirmed_balance: None,
            confirmed_pending: None,
            confirmed_receivable: None,
            confirmed_representative: None,
            confirmed_frontier: None,
            confirmation_height: None,
        };

        if include_confirmed {
            let confirmed_balance = if info.block_count != confirmation_height_info.height {
                self.node
                    .ledger
                    .any()
                    .block_balance(&txn, &confirmation_height_info.frontier)
                    .unwrap_or(Amount::zero())
            } else {
                // block_height and confirmed height are the same, so can just reuse balance
                info.balance
            };
            account_info.confirmed_balance = Some(confirmed_balance);
            account_info.confirmed_height = Some(confirmation_height_info.height.into());
            account_info.confirmation_height = Some(confirmation_height_info.height.into());
            account_info.confirmed_frontier = Some(confirmation_height_info.frontier);
        } else {
            // For backwards compatibility purposes
            account_info.confirmation_height = Some(confirmation_height_info.height.into());
            account_info.confirmed_height = Some(confirmation_height_info.height.into());
            account_info.confirmation_height_frontier = Some(confirmation_height_info.frontier);
        }

        if unwrap_bool_or_false(args.representative) {
            account_info.representative = Some(info.representative.into());
            if include_confirmed {
                let confirmed_representative = if confirmation_height_info.height > 0 {
                    if let Some(confirmed_frontier_block) = self
                        .node
                        .ledger
                        .any()
                        .get_block(&txn, &confirmation_height_info.frontier)
                    {
                        confirmed_frontier_block
                            .representative_field()
                            .unwrap_or_else(|| {
                                let rep_block_hash = self.node.ledger.representative_block_hash(
                                    &txn,
                                    &confirmation_height_info.frontier,
                                );
                                self.node
                                    .ledger
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

        if unwrap_bool_or_false(args.weight) {
            account_info.weight = Some(self.node.ledger.weight_exact(&txn, args.account.into()));
        }

        let receivable = unwrap_bool_or_false(args.receivable);
        if receivable {
            let account_receivable =
                self.node
                    .ledger
                    .account_receivable(&txn, &args.account, false);
            account_info.pending = Some(account_receivable);
            account_info.receivable = Some(account_receivable);

            if include_confirmed {
                let confirmed_receivable =
                    self.node
                        .ledger
                        .account_receivable(&txn, &args.account, true);
                account_info.confirmed_pending = Some(confirmed_receivable);
                account_info.confirmed_receivable = Some(confirmed_receivable);
            }
        }

        Ok(account_info)
    }
}

fn epoch_as_number(epoch: Epoch) -> u8 {
    match epoch {
        Epoch::Epoch1 => 1,
        Epoch::Epoch2 => 2,
        _ => 0,
    }
}
