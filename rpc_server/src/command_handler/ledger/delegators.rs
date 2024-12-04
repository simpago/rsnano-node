use crate::command_handler::RpcCommandHandler;
use rsnano_core::{Account, Amount, PublicKey};
use rsnano_rpc_messages::{unwrap_u64_or, DelegatorsArgs, DelegatorsResponse};
use std::collections::HashMap;

impl RpcCommandHandler {
    pub(crate) fn delegators(&self, args: DelegatorsArgs) -> DelegatorsResponse {
        let representative: PublicKey = args.account.into();
        let count = unwrap_u64_or(args.count, 1024);
        let threshold = args.threshold.unwrap_or(Amount::zero());
        let start_account = args.start.unwrap_or(Account::zero());

        let mut delegators: HashMap<Account, Amount> = HashMap::new();
        let tx = self.node.ledger.read_txn();
        let mut iter = self
            .node
            .store
            .account
            .begin_account(&tx, &start_account.inc().unwrap_or_default());

        while let Some((account, info)) = iter.current() {
            if delegators.len() >= count as usize {
                break;
            }

            if info.representative == representative && info.balance >= threshold {
                delegators.insert(*account, info.balance);
            }

            iter.next();
        }
        DelegatorsResponse::new(delegators)
    }
}
