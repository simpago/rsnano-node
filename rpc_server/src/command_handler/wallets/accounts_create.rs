use crate::command_handler::RpcCommandHandler;
use rsnano_core::Account;
use rsnano_node::wallets::WalletsExt;
use rsnano_rpc_messages::{unwrap_bool_or_false, AccountsCreateArgs, AccountsRpcMessage};

impl RpcCommandHandler {
    pub(crate) fn accounts_create(
        &self,
        args: AccountsCreateArgs,
    ) -> anyhow::Result<AccountsRpcMessage> {
        let generate_work = unwrap_bool_or_false(args.work);
        let count: usize = args.count.into();
        let wallet = &args.wallet;

        let accounts: Result<Vec<Account>, _> = (0..count)
            .map(|_| {
                self.node
                    .wallets
                    .deterministic_insert2(wallet, generate_work)
                    .map(|key| Account::from(key))
            })
            .collect();

        let accounts = accounts?;
        Ok(AccountsRpcMessage::new(accounts))
    }
}
