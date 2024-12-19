use crate::command_handler::RpcCommandHandler;
use rsnano_core::utils::UnixTimestamp;
use rsnano_core::Account;
use rsnano_node::Node;
use rsnano_rpc_messages::{AccountInfo, WalletLedgerArgs, WalletLedgerResponse};
use std::collections::HashMap;
use std::sync::Arc;

impl RpcCommandHandler {
    pub(crate) fn wallet_ledger(
        &self,
        args: WalletLedgerArgs,
    ) -> anyhow::Result<WalletLedgerResponse> {
        let representative = args.representative.unwrap_or_default().inner();
        let weight = args.weight.unwrap_or_default().inner();
        let receivable = args.receivable.unwrap_or_default().inner();
        let modified_since = args.modified_since.unwrap_or_default().inner();

        let accounts = self.node.wallets.get_accounts_of_wallet(&args.wallet)?;
        let account_dtos = get_accounts_info(
            self.node.clone(),
            accounts,
            representative,
            weight,
            receivable,
            modified_since.into(),
        );
        Ok(WalletLedgerResponse {
            accounts: account_dtos,
        })
    }
}

fn get_accounts_info(
    node: Arc<Node>,
    accounts: Vec<Account>,
    representative: bool,
    weight: bool,
    receivable: bool,
    modified_since: UnixTimestamp,
) -> HashMap<Account, AccountInfo> {
    let tx = node.store.tx_begin_read();
    let mut account_dtos = HashMap::new();

    for account in accounts {
        if let Some(info) = node.ledger.any().get_account(&tx, &account) {
            if info.modified >= modified_since {
                let entry = AccountInfo {
                    frontier: info.head,
                    open_block: info.open_block,
                    representative_block: node.ledger.representative_block_hash(&tx, &info.head),
                    balance: info.balance,
                    modified_timestamp: info.modified.as_u64().into(),
                    block_count: info.block_count.into(),
                    representative: representative.then(|| info.representative.as_account()),
                    weight: weight.then(|| node.ledger.weight_exact(&tx, account.into())),
                    receivable: receivable
                        .then(|| node.ledger.account_receivable(&tx, &account, false)),
                    pending: receivable
                        .then(|| node.ledger.account_receivable(&tx, &account, false)),
                };

                account_dtos.insert(account, entry);
            }
        }
    }

    account_dtos
}
