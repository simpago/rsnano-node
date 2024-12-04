use crate::command_handler::RpcCommandHandler;
use indexmap::IndexMap;
use rsnano_core::BlockHash;
use rsnano_rpc_messages::{
    AccountsReceivableResponse, AccountsReceivableSimple, AccountsReceivableSource,
    AccountsReceivableThreshold, SourceInfo, WalletReceivableArgs,
};

impl RpcCommandHandler {
    pub(crate) fn wallet_receivable(
        &self,
        args: WalletReceivableArgs,
    ) -> anyhow::Result<AccountsReceivableResponse> {
        let count = args.count.unwrap_or(usize::MAX.into()).inner();
        let threshold = args.threshold.unwrap_or_default();
        let source = args.source.unwrap_or_default().inner();
        let min_version = args.min_version.unwrap_or_default().inner();
        let include_only_confirmed = args.include_only_confirmed.unwrap_or(true.into()).inner();

        let accounts = self.node.wallets.get_accounts_of_wallet(&args.wallet)?;
        let tx = self.node.ledger.read_txn();

        let mut pending_source = IndexMap::new();
        let mut pending_threshold = IndexMap::new();
        let mut pending_default = IndexMap::new();

        for account in accounts {
            let mut block_source = IndexMap::new();
            let mut block_threshold = IndexMap::new();
            let mut block_default = Vec::new();

            for (key, info) in self
                .node
                .ledger
                .any()
                .account_receivable_upper_bound(&tx, account, BlockHash::zero())
                .take(count)
            {
                if include_only_confirmed
                    && !self
                        .node
                        .ledger
                        .confirmed()
                        .block_exists_or_pruned(&tx, &key.send_block_hash)
                {
                    continue;
                }

                if threshold.is_zero() && !source {
                    block_default.push(key.send_block_hash);
                    continue;
                }

                if source || min_version {
                    block_source.insert(
                        key.send_block_hash,
                        SourceInfo {
                            amount: info.amount,
                            source: source.then(|| info.source),
                            min_version: min_version.then(|| info.epoch.epoch_number().into()),
                        },
                    );
                } else {
                    block_threshold.insert(key.send_block_hash, info.amount);
                }
            }
            if !block_source.is_empty() {
                pending_source.insert(account, block_source);
            } else if !block_threshold.is_empty() {
                pending_threshold.insert(account, block_threshold);
            } else if !block_default.is_empty() {
                pending_default.insert(account, block_default);
            }
        }

        if threshold.is_zero() && !source {
            Ok(AccountsReceivableResponse::Simple(
                AccountsReceivableSimple {
                    blocks: pending_default,
                },
            ))
        } else if source || min_version {
            Ok(AccountsReceivableResponse::Source(
                AccountsReceivableSource {
                    blocks: pending_source,
                },
            ))
        } else {
            Ok(AccountsReceivableResponse::Threshold(
                AccountsReceivableThreshold {
                    blocks: pending_threshold,
                },
            ))
        }
    }
}
