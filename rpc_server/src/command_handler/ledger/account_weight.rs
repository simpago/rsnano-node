use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::{AccountWeightArgs, WeightDto};

impl RpcCommandHandler {
    pub(crate) fn account_weight(&self, args: AccountWeightArgs) -> WeightDto {
        let tx = self.node.ledger.read_txn();
        let weight = self.node.ledger.weight_exact(&tx, args.account.into());
        WeightDto::new(weight)
    }
}
