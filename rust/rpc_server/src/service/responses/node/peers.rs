use rsnano_node::node::Node;
use rsnano_rpc_messages::{PeerData, PeerInfo, PeersDto};
use std::{collections::HashMap, sync::Arc};

pub async fn peers(node: Arc<Node>, peer_details: Option<bool>) -> String {
    let peer_details = peer_details.unwrap_or(false);
    let mut peers: HashMap<String, PeerInfo> = HashMap::new();

    node.network_info
        .read()
        .unwrap()
        .random_realtime_channels(usize::MAX, 0)
        .iter()
        .for_each(|channel| {
            peers.insert(
                channel.ipv4_address_or_ipv6_subnet().to_string(),
                PeerInfo::Detailed {
                    protocol_version: channel.protocol_version(),
                    node_id: channel.node_id().unwrap().into(),
                    connection_type: "tcp".to_string(),
                },
            );
        });

    let peer_data = if peer_details {
        PeerData::Detailed(peers)
    } else {
        PeerData::Simple(peers.keys().cloned().collect())
    };

    let peer_dto = PeersDto::new(peer_data);

    serde_json::to_string_pretty(&peer_dto).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn peers_without_details() {
        let mut system = System::new();
        let node1 = system.make_node();
        let _node2 = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node1.clone(), false);

        let result = node1
            .tokio
            .block_on(async { rpc_client.peers(None).await.unwrap() });

        match result.peers {
            PeerData::Simple(peers) => {
                assert!(!peers.is_empty());
            }
            PeerData::Detailed(_) => panic!("Expected Simple peer data"),
        }

        server.abort();
    }

    #[test]
    fn peers_with_details() {
        let mut system = System::new();
        let node1 = system.make_node();
        let _node2 = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node1.clone(), false);

        let result = node1
            .tokio
            .block_on(async { rpc_client.peers(Some(true)).await.unwrap() });

        println!("{:?}", result);

        match result.peers {
            PeerData::Detailed(peers) => {
                assert!(!peers.is_empty());
            }
            PeerData::Simple(_) => panic!("Expected Detailed peer data"),
        }

        server.abort();
    }
}
