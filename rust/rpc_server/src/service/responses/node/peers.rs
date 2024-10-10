use rsnano_node::Node;
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
