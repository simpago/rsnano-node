use crate::command_handler::RpcCommandHandler;
use indexmap::IndexMap;
use rsnano_core::{Account, Amount};
use rsnano_rpc_messages::{
    unwrap_bool_or_false, unwrap_u64_or_max, RepresentativesArgs, RepresentativesResponse,
};

impl RpcCommandHandler {
    pub(crate) fn representatives(&self, args: RepresentativesArgs) -> RepresentativesResponse {
        let count = unwrap_u64_or_max(args.count) as usize;
        let sorting = unwrap_bool_or_false(args.sorting);
        let representatives = if sorting {
            let mut representatives: IndexMap<Account, Amount> = self
                .node
                .ledger
                .rep_weights
                .read()
                .iter()
                .map(|(pk, amount)| (Account::from(pk), *amount))
                .collect();

            representatives.sort_by(|_, v1, _, v2| v2.cmp(v1));
            representatives.truncate(count);
            representatives
        } else {
            self.node
                .ledger
                .rep_weights
                .read()
                .iter()
                .map(|(k, w)| (Account::from(k), *w))
                .take(count)
                .collect()
        };

        RepresentativesResponse::new(representatives)
    }
}
