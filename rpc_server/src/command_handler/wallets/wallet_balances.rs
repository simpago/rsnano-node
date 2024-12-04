use crate::command_handler::RpcCommandHandler;
use rsnano_core::Amount;
use rsnano_rpc_messages::{AccountBalanceResponse, AccountsBalancesResponse, WalletBalancesArgs};
use std::collections::HashMap;

impl RpcCommandHandler {
    pub(crate) fn wallet_balances(&self, args: WalletBalancesArgs) -> AccountsBalancesResponse {
        let threshold = args.threshold.unwrap_or(Amount::zero());
        let accounts = self
            .node
            .wallets
            .get_accounts_of_wallet(&args.wallet)
            .unwrap();
        let mut balances = HashMap::new();
        let tx = self.node.ledger.read_txn();
        for account in accounts {
            let balance = self
                .node
                .ledger
                .any()
                .account_balance(&tx, &account)
                .unwrap_or_default();

            if balance >= threshold {
                let pending = self.node.ledger.account_receivable(&tx, &account, false);

                let account_balance = AccountBalanceResponse {
                    balance,
                    pending,
                    receivable: pending,
                };
                balances.insert(account, account_balance);
            }
        }
        AccountsBalancesResponse { balances }
    }
}
