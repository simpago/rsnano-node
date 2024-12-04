use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::{AccountArg, AccountRepresentativeDto};

impl RpcCommandHandler {
    pub(crate) fn account_representative(
        &self,
        args: AccountArg,
    ) -> anyhow::Result<AccountRepresentativeDto> {
        let tx = self.node.ledger.read_txn();
        let account_info = self.load_account(&tx, &args.account)?;
        Ok(AccountRepresentativeDto::new(
            account_info.representative.as_account(),
        ))
    }
}
