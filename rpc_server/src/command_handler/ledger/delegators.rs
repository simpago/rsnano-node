use crate::command_handler::RpcCommandHandler;
use rsnano_core::{Account, Amount, PublicKey};
use rsnano_rpc_messages::{unwrap_u64_or, DelegatorsArgs, DelegatorsResponse};

impl RpcCommandHandler {
    pub(crate) fn delegators(&self, args: DelegatorsArgs) -> DelegatorsResponse {
        let representative: PublicKey = args.account.into();
        let count = unwrap_u64_or(args.count, 1024) as usize;
        let threshold = args.threshold.unwrap_or(Amount::zero());

        let start_account = args
            .start
            .unwrap_or(Account::zero())
            .inc()
            .unwrap_or_default();

        let tx = self.node.ledger.read_txn();

        let delegators = self
            .node
            .store
            .account
            .iter_range(&tx, start_account..)
            .filter_map(|(account, info)| {
                if info.representative == representative && info.balance >= threshold {
                    Some((account, info.balance))
                } else {
                    None
                }
            })
            .take(count);

        DelegatorsResponse::new(delegators.collect())
    }
}
