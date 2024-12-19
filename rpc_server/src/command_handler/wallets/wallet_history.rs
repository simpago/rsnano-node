use crate::command_handler::{ledger::AccountHistoryHelper, RpcCommandHandler};
use rsnano_core::{utils::UnixTimestamp, Account, BlockHash};
use rsnano_rpc_messages::{HistoryEntry, WalletHistoryArgs, WalletHistoryResponse};

impl RpcCommandHandler {
    pub(crate) fn wallet_history(
        &self,
        args: WalletHistoryArgs,
    ) -> anyhow::Result<WalletHistoryResponse> {
        let modified_since: UnixTimestamp = args.modified_since.unwrap_or(1.into()).inner().into();
        let accounts = self.node.wallets.get_accounts_of_wallet(&args.wallet)?;
        let mut entries: Vec<HistoryEntry> = Vec::new();
        let tx = self.node.store.tx_begin_read();

        for account in accounts {
            if let Some(info) = self.node.ledger.any().get_account(&tx, &account) {
                let mut timestamp = info.modified;
                let mut hash = info.head;

                while timestamp >= modified_since && !hash.is_zero() {
                    if let Some(block) = self.node.ledger.any().get_block(&tx, &hash) {
                        timestamp = block.timestamp().into();

                        let helper = AccountHistoryHelper {
                            ledger: &self.node.ledger,
                            accounts_to_filter: Vec::new(),
                            reverse: false,
                            offset: 0,
                            head: None,
                            requested_account: Some(account),
                            output_raw: false,
                            count: u64::MAX,
                            current_block_hash: BlockHash::zero(),
                            account: Account::zero(),
                        };

                        let entry = helper.entry_for(&block, &tx);

                        if let Some(mut entry) = entry {
                            entry.block_account = Some(account);
                            entries.push(entry);
                        }

                        hash = block.previous();
                    } else {
                        hash = BlockHash::zero()
                    }
                }
            }
        }

        entries.sort_by(|a, b| b.local_timestamp.cmp(&a.local_timestamp));
        Ok(WalletHistoryResponse::new(entries))
    }
}
