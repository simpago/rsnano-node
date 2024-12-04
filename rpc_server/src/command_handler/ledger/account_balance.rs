use crate::command_handler::RpcCommandHandler;
use rsnano_core::Amount;
use rsnano_rpc_messages::{
    unwrap_bool_or_true, AccountArg, AccountBalanceArgs, AccountBalanceResponse,
    AccountBlockCountResponse,
};

impl RpcCommandHandler {
    pub(crate) fn account_balance(&self, args: AccountBalanceArgs) -> AccountBalanceResponse {
        let only_confirmed = unwrap_bool_or_true(args.include_only_confirmed);

        let tx = self.node.ledger.read_txn();
        let balance = if only_confirmed {
            self.node
                .ledger
                .confirmed()
                .account_balance(&tx, &args.account)
                .unwrap_or(Amount::zero())
        } else {
            self.node
                .ledger
                .any()
                .account_balance(&tx, &args.account)
                .unwrap_or(Amount::zero())
        };

        let receivable = self
            .node
            .ledger
            .account_receivable(&tx, &args.account, only_confirmed);

        AccountBalanceResponse {
            balance,
            pending: receivable,
            receivable,
        }
    }

    pub(crate) fn account_block_count(
        &self,
        args: AccountArg,
    ) -> anyhow::Result<AccountBlockCountResponse> {
        let tx = self.node.ledger.read_txn();
        let account = self.load_account(&tx, &args.account)?;
        Ok(AccountBlockCountResponse::new(account.block_count))
    }
}
