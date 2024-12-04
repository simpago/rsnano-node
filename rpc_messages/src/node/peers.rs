use crate::{RpcBool, RpcCommand, RpcU8};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddrV6};

impl RpcCommand {
    pub fn peers(peer_details: Option<bool>) -> Self {
        Self::Peers(PeersArgs::new(peer_details))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct PeersArgs {
    pub peer_details: Option<RpcBool>,
}

impl PeersArgs {
    pub fn new(peer_details: Option<bool>) -> Self {
        PeersArgs {
            peer_details: peer_details.map(|i| i.into()),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct PeerInfo {
    pub protocol_version: RpcU8,
    pub node_id: String,
    #[serde(rename = "type")]
    pub connection_type: String,
    pub peering: SocketAddrV6,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PeersDto {
    Simple(SimplePeers),
    Detailed(DetailedPeers),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimplePeers {
    pub peers: HashMap<SocketAddrV6, RpcU8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DetailedPeers {
    pub peers: HashMap<SocketAddrV6, PeerInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;
    use std::net::Ipv6Addr;

    #[test]
    fn serialize_simple_peers() {
        let simple_peers = PeersDto::Simple(SimplePeers {
            peers: [("[::ffff:172.17.0.1]:32841".parse().unwrap(), 16.into())].into(),
        });

        let json = serde_json::to_string(&simple_peers).unwrap();
        assert_eq!(json, r#"{"peers":{"[::ffff:172.17.0.1]:32841":"16"}}"#);
    }

    #[test]
    fn deserialize_simple_peers() {
        let json = r#"{"peers":{"[::ffff:172.17.0.1]:32841": "16"}}"#;
        let peers: SimplePeers = serde_json::from_str(json).unwrap();

        assert_eq!(peers.peers.len(), 1);
    }

    #[test]
    fn serialize_detailed_peers() {
        let mut peers = HashMap::new();
        peers.insert(
            "[::ffff:172.17.0.1]:7075".parse().unwrap(),
            PeerInfo {
                protocol_version: 18.into(),
                node_id: "node_1y7j5rdqhg99uyab1145gu3yur1ax35a3b6qr417yt8cd6n86uiw3d4whty3"
                    .to_string(),
                connection_type: "tcp".to_string(),
                peering: SocketAddrV6::new(Ipv6Addr::LOCALHOST, 111, 0, 0),
            },
        );

        let peers = PeersDto::Detailed(DetailedPeers { peers });

        let json = serde_json::to_string(&peers).unwrap();
        assert_eq!(
            json,
            r#"{"peers":{"[::ffff:172.17.0.1]:7075":{"protocol_version":"18","node_id":"node_1y7j5rdqhg99uyab1145gu3yur1ax35a3b6qr417yt8cd6n86uiw3d4whty3","type":"tcp","peering":"[::1]:111"}}}"#
        );
    }

    #[test]
    fn deserialize_detailed_peers() {
        let json = r#"{"peers":{"[::ffff:172.17.0.1]:7075":{"protocol_version":"18","node_id":"node_1y7j5rdqhg99uyab1145gu3yur1ax35a3b6qr417yt8cd6n86uiw3d4whty3","type":"tcp","peering":"[::1]:111"}}}"#;
        let peers: DetailedPeers = serde_json::from_str(json).unwrap();

        assert_eq!(peers.peers.len(), 1);
        let peer_info = peers
            .peers
            .get(&"[::ffff:172.17.0.1]:7075".parse().unwrap())
            .unwrap();
        assert_eq!(peer_info.protocol_version, 18.into());
        assert_eq!(
            peer_info.node_id,
            "node_1y7j5rdqhg99uyab1145gu3yur1ax35a3b6qr417yt8cd6n86uiw3d4whty3"
        );
        assert_eq!(peer_info.connection_type, "tcp");
    }
}
