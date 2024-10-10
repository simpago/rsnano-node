use rsnano_core::{Amount, QualifiedRoot};
use rsnano_node::Node;
use rsnano_rpc_messages::{ConfirmationBlockInfoDto, ConfirmationInfoDto, ErrorDto};
use serde_json::to_string_pretty;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn confirmation_info(
    node: Arc<Node>,
    root: QualifiedRoot,
    contents: Option<bool>,
    representatives: Option<bool>,
) -> String {
    let election = node.active.election(&root);

    if let Some(election) = election {
        if !node.active.confirmed(&election) {
            let info = election.mutex.lock().unwrap();
            let mut blocks = HashMap::new();
            let mut total_tally = Amount::zero();

            for (hash, block) in info.last_blocks.iter() {
                let tally = info.last_tally.get(hash).cloned().unwrap_or(Amount::zero());
                let mut block_info = ConfirmationBlockInfoDto {
                    tally,
                    contents: None,
                    representatives: None,
                };

                if contents.unwrap_or(true) {
                    block_info.contents = Some(block.json_representation());
                }

                if representatives.unwrap_or(false) {
                    let mut reps = HashMap::new();
                    for (representative, vote) in &info.last_votes {
                        if hash == &vote.hash {
                            let amount = node.ledger.rep_weights.weight(representative);
                            reps.insert(representative.clone().into(), amount);
                        }
                    }
                    block_info.representatives = Some(reps);
                }

                total_tally += tally;
                blocks.insert(*hash, block_info);
            }

            let confirmation_info_dto = ConfirmationInfoDto::new(
                info.status.confirmation_request_count,
                info.last_votes.len(),
                info.status
                    .winner
                    .as_ref()
                    .map(|w| w.hash())
                    .unwrap_or_default(),
                total_tally,
                info.status.final_tally,
                blocks,
            );

            to_string_pretty(&confirmation_info_dto).unwrap()
        } else {
            to_string_pretty(&ErrorDto::new("Confirmation not found".to_string())).unwrap()
        }
    } else {
        to_string_pretty(&ErrorDto::new("Invalid root".to_string())).unwrap()
    }
}
