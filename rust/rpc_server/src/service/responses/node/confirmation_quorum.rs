use rsnano_core::utils::NULL_ENDPOINT;
use rsnano_node::node::Node;
use rsnano_rpc_messages::{ConfirmationQuorumDto, PeerDetailsDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn confirmation_quorum(node: Arc<Node>, peer_details: Option<bool>) -> String {
    let quorum = node.online_reps.lock().unwrap();

    let mut confirmation_quorum_dto = ConfirmationQuorumDto {
        quorum_delta: quorum.quorum_delta(),
        online_weight_quorum_percent: quorum.quorum_percent(),
        online_weight_minimum: quorum.online_weight_minimum(),
        online_stake_total: quorum.online_weight(),
        trended_stake_total: quorum.trended_weight(),
        peers_stake_total: quorum.peered_weight(),
        peers: None,
    };

    if peer_details.unwrap_or(false) {
        let peers = quorum
            .peered_reps()
            .iter()
            .map(|rep| {
                let endpoint = node
                    .network_info
                    .read()
                    .unwrap()
                    .get(rep.channel_id)
                    .map(|c| c.peer_addr())
                    .unwrap_or(NULL_ENDPOINT);

                PeerDetailsDto {
                    account: rep.account.into(),
                    ip: endpoint,
                    weight: node.ledger.weight(&rep.account),
                }
            })
            .collect();

        confirmation_quorum_dto.peers = Some(peers);
    }

    to_string_pretty(&confirmation_quorum_dto).unwrap()
}

#[cfg(test)]
mod tests {
    use rsnano_core::{Amount, WalletId, DEV_GENESIS_KEY};
    use rsnano_node::{config::NodeFlags, wallets::WalletsExt};
    use rsnano_rpc_messages::{ConfirmationQuorumDto, PeerDetailsDto};
    use test_helpers::{establish_tcp, send_block, setup_rpc_client_and_server, System};

    #[test]
    fn confirmation_quorum() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let result = node
            .tokio
            .block_on(async { rpc_client.confirmation_quorum(None).await.unwrap() });

        let reps = node.online_reps.lock().unwrap();

        assert_eq!(result.quorum_delta, reps.quorum_delta());
        assert_eq!(result.online_weight_quorum_percent, reps.quorum_percent());
        assert_eq!(result.online_weight_minimum, reps.online_weight_minimum());
        assert_eq!(result.online_stake_total, reps.online_weight());
        assert_eq!(result.peers_stake_total, reps.peered_weight());
        assert_eq!(result.trended_stake_total, reps.trended_weight());
        assert_eq!(result.peers, None);

        server.abort();
    }

    #[test]
    fn confirmation_quorum_peer_details() {
        let mut system = System::new();

        let node0 = system.make_node();

        let mut node1_config = System::default_config();
        node1_config.tcp_incoming_connections_max = 0; // Prevent ephemeral node1->node0 channel replacement with incoming connection
        let node1 = system
            .build_node()
            .config(node1_config)
            .disconnected()
            .finish();

        let channel1 = establish_tcp(&node1, &node0);

        let wallet_id = WalletId::zero();
        node1.wallets.create(wallet_id);
        node1
            .wallets
            .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), false)
            .unwrap();

        let hash = send_block(node0.clone());

        node0.confirm(hash);

        assert_eq!(node0.block_confirmed(&hash), true);

        let (rpc_client, server) = setup_rpc_client_and_server(node1.clone(), false);

        let result = node0
            .tokio
            .block_on(async { rpc_client.confirmation_quorum(Some(true)).await.unwrap() });

        let reps = node0.online_reps.lock().unwrap();

        assert_eq!(result.quorum_delta, reps.quorum_delta());
        assert_eq!(result.online_weight_quorum_percent, reps.quorum_percent());
        assert_eq!(result.online_weight_minimum, reps.online_weight_minimum());
        assert_eq!(result.online_stake_total, reps.online_weight());
        assert_eq!(result.peers_stake_total, reps.peered_weight());
        assert_eq!(result.trended_stake_total, reps.trended_weight());

        let peer_details = result.peers.unwrap();
        println!("{:?}", peer_details);

        server.abort();
    }
}
