use crate::command_handler::RpcCommandHandler;
use rsnano_core::Account;
use rsnano_rpc_messages::{
    DetailedRepresentativesOnline, RepWeightDto, RepresentativesOnlineArgs,
    RepresentativesOnlineResponse, SimpleRepresentativesOnline,
};
use std::collections::HashMap;

impl RpcCommandHandler {
    pub(crate) fn representatives_online(
        &self,
        args: RepresentativesOnlineArgs,
    ) -> RepresentativesOnlineResponse {
        let lock = self.node.online_reps.lock().unwrap();
        let online_reps = lock.online_reps();
        let weight = args.weight.unwrap_or_default().inner();

        let mut representatives_simple = Vec::new();
        let mut representatives_detailed = HashMap::new();

        let filtering = args.accounts.is_some();
        let mut accounts_to_filter = args.accounts.unwrap_or_default();

        for rep in online_reps {
            let account = Account::from(rep);

            if filtering {
                if accounts_to_filter.is_empty() {
                    break;
                }

                if !accounts_to_filter.contains(&account) {
                    continue;
                }
                accounts_to_filter.retain(|a| *a != account);
            }

            if weight {
                let weight = self.node.ledger.weight(rep);
                representatives_detailed.insert(account, RepWeightDto { weight });
            } else {
                representatives_simple.push(account);
            };
        }

        if weight {
            RepresentativesOnlineResponse::Detailed(DetailedRepresentativesOnline {
                representatives: representatives_detailed,
            })
        } else {
            RepresentativesOnlineResponse::Simple(SimpleRepresentativesOnline {
                representatives: representatives_simple,
            })
        }
    }
}
