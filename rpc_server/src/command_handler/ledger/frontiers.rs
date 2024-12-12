use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::{FrontiersArgs, FrontiersResponse};

impl RpcCommandHandler {
    pub(crate) fn frontiers(&self, args: FrontiersArgs) -> FrontiersResponse {
        let tx = self.node.ledger.read_txn();

        let frontiers = self
            .node
            .store
            .account
            .iter_range(&tx, args.account..)
            .map(|(account, info)| (account, info.head))
            .take(args.count.into());

        FrontiersResponse::new(frontiers.collect())
    }
}
