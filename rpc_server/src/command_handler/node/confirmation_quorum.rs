use crate::command_handler::RpcCommandHandler;
use rsnano_core::utils::NULL_ENDPOINT;
use rsnano_rpc_messages::{ConfirmationQuorumArgs, ConfirmationQuorumResponse, PeerDetailsDto};

impl RpcCommandHandler {
    pub(crate) fn confirmation_quorum(
        &self,
        args: ConfirmationQuorumArgs,
    ) -> ConfirmationQuorumResponse {
        let quorum = self.node.online_reps.lock().unwrap();

        let mut result = ConfirmationQuorumResponse {
            quorum_delta: quorum.quorum_delta(),
            online_weight_quorum_percent: quorum.quorum_percent().into(),
            online_weight_minimum: quorum.online_weight_minimum(),
            online_stake_total: quorum.online_weight(),
            trended_stake_total: quorum.trended_weight_or_minimum_online_weight(),
            peers_stake_total: quorum.peered_weight(),
            peers: None,
        };

        if args.peer_details.unwrap_or_default().inner() {
            let peers = quorum
                .peered_reps()
                .iter()
                .map(|rep| {
                    let endpoint = self
                        .node
                        .network_info
                        .read()
                        .unwrap()
                        .get(rep.channel_id)
                        .map(|c| c.peer_addr())
                        .unwrap_or(NULL_ENDPOINT);

                    PeerDetailsDto {
                        account: rep.account.into(),
                        ip: endpoint,
                        weight: self.node.ledger.weight(&rep.account),
                    }
                })
                .collect();

            result.peers = Some(peers);
        }

        result
    }
}
