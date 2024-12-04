use crate::command_handler::RpcCommandHandler;
use rsnano_core::Amount;
use rsnano_rpc_messages::{
    unwrap_bool_or_true, AccountBalanceResponse, AccountsBalancesArgs, AccountsBalancesResponse,
};
use std::collections::HashMap;

impl RpcCommandHandler {
    pub(crate) fn accounts_balances(&self, args: AccountsBalancesArgs) -> AccountsBalancesResponse {
        let tx = self.node.ledger.read_txn();
        let mut balances = HashMap::new();
        let only_confirmed = unwrap_bool_or_true(args.include_only_confirmed);

        for account in args.accounts {
            let balance = if only_confirmed {
                self.node
                    .ledger
                    .confirmed()
                    .account_balance(&tx, &account)
                    .unwrap_or(Amount::zero())
            } else {
                self.node
                    .ledger
                    .any()
                    .account_balance(&tx, &account)
                    .unwrap_or(Amount::zero())
            };

            let pending = self
                .node
                .ledger
                .account_receivable(&tx, &account, only_confirmed);

            balances.insert(
                account,
                AccountBalanceResponse {
                    balance,
                    pending,
                    receivable: pending,
                },
            );
        }

        AccountsBalancesResponse { balances }
    }
}
