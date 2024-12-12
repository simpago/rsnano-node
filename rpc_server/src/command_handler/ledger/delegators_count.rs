use crate::command_handler::RpcCommandHandler;
use rsnano_core::PublicKey;
use rsnano_rpc_messages::{AccountArg, CountResponse};

impl RpcCommandHandler {
    pub(crate) fn delegators_count(&self, args: AccountArg) -> CountResponse {
        let representative: PublicKey = args.account.into();
        let tx = self.node.ledger.read_txn();

        let count = self
            .node
            .store
            .account
            .iter(&tx)
            .filter(|(_, info)| info.representative == representative)
            .count();

        CountResponse::new(count as u64)
    }
}
