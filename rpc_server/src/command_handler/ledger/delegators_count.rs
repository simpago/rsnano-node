use crate::command_handler::RpcCommandHandler;
use rsnano_core::PublicKey;
use rsnano_rpc_messages::{AccountArg, CountResponse};

impl RpcCommandHandler {
    pub(crate) fn delegators_count(&self, args: AccountArg) -> CountResponse {
        let representative: PublicKey = args.account.into();
        let mut count = 0;

        let tx = self.node.ledger.read_txn();
        let mut iter = self.node.store.account.begin(&tx);

        while let Some((_, info)) = iter.current() {
            if info.representative == representative {
                count += 1;
            }

            iter.next();
        }
        CountResponse::new(count)
    }
}
