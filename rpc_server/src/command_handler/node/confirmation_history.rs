use crate::command_handler::RpcCommandHandler;
use rsnano_rpc_messages::{
    ConfirmationEntry, ConfirmationHistoryArgs, ConfirmationHistoryResponse, ConfirmationStats,
};
use std::time::{Duration, UNIX_EPOCH};

impl RpcCommandHandler {
    pub(crate) fn confirmation_history(
        &self,
        args: ConfirmationHistoryArgs,
    ) -> ConfirmationHistoryResponse {
        let mut elections = Vec::new();
        let mut running_total = Duration::ZERO;
        let hash = args.hash.unwrap_or_default();
        for status in self.node.active.recently_cemented_list() {
            if hash.is_zero() || status.winner.as_ref().unwrap().hash() == hash {
                elections.push(ConfirmationEntry {
                    hash: status.winner.as_ref().unwrap().hash(),
                    duration: status.election_duration.as_secs().into(),
                    time: (status
                        .election_end
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64)
                        .into(),
                    tally: status.tally,
                    final_tally: status.final_tally,
                    blocks: status.block_count.into(),
                    voters: status.voter_count.into(),
                    request_count: status.confirmation_request_count.into(),
                });
            }
            running_total += status.election_duration;
        }

        ConfirmationHistoryResponse {
            confirmation_stats: ConfirmationStats {
                count: elections.len().into(),
                average: if elections.is_empty() {
                    None
                } else {
                    Some((running_total.as_secs() / elections.len() as u64).into())
                },
            },
            confirmations: elections,
        }
    }
}
