use crate::RpcCommand;
use rsnano_core::Account;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

impl RpcCommand {
    pub fn peers(peer_details: Option<bool>) -> Self {
        Self::Peers(PeersArgs::new(peer_details))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct PeersArgs {
    pub peer_details: Option<bool>,
}

impl PeersArgs {
    pub fn new(peer_details: Option<bool>) -> Self {
        PeersArgs { peer_details }
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PeerInfo {
    Simple(String),
    Detailed {
        protocol_version: u8,
        #[serde(
            serialize_with = "serialize_node_id",
            deserialize_with = "deserialize_node_id"
        )]
        node_id: Account,
        #[serde(rename = "type")]
        connection_type: String,
    },
}

fn serialize_node_id<S>(account: &Account, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&account.to_node_id())
}

fn deserialize_node_id<'de, D>(deserializer: D) -> Result<Account, D::Error>
where
    D: Deserializer<'de>,
{
    let node_id_str = String::deserialize(deserializer)?;
    let account_str = node_id_str.replacen("node", "nano", 1);
    Account::decode_account(&account_str).map_err(serde::de::Error::custom)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PeersDto {
    pub peers: PeerData,
}

impl PeersDto {
    pub fn new(peers: PeerData) -> Self {
        PeersDto { peers }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PeerData {
    Simple(Vec<String>),
    Detailed(HashMap<String, PeerInfo>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn serialize_simple_peers() {
        let simple_peers = PeersDto {
            peers: PeerData::Simple(vec!["[::ffff:172.17.0.1]:32841".to_string()]),
        };

        let json = serde_json::to_string(&simple_peers).unwrap();
        assert_eq!(json, r#"{"peers":["[::ffff:172.17.0.1]:32841"]}"#);
    }

    #[test]
    fn deserialize_simple_peers() {
        let json = r#"{"peers":["[::ffff:172.17.0.1]:32841"]}"#;
        let peers: PeersDto = serde_json::from_str(json).unwrap();

        match peers.peers {
            PeerData::Simple(vec) => {
                assert_eq!(vec.len(), 1);
                assert_eq!(vec[0], "[::ffff:172.17.0.1]:32841");
            }
            PeerData::Detailed(_) => panic!("Expected Simple, got Detailed"),
        }
    }

    #[test]
    fn serialize_detailed_peers() {
        let mut detailed_peers = HashMap::new();
        detailed_peers.insert(
            "[::ffff:172.17.0.1]:7075".to_string(),
            PeerInfo::Detailed {
                protocol_version: 18,
                node_id: Account::decode_account(
                    "nano_1y7j5rdqhg99uyab1145gu3yur1ax35a3b6qr417yt8cd6n86uiw3d4whty3",
                )
                .unwrap(),
                connection_type: "tcp".to_string(),
            },
        );

        let peers = PeersDto {
            peers: PeerData::Detailed(detailed_peers),
        };

        let json = serde_json::to_string(&peers).unwrap();
        assert_eq!(
            json,
            r#"{"peers":{"[::ffff:172.17.0.1]:7075":{"protocol_version":18,"node_id":"node_1y7j5rdqhg99uyab1145gu3yur1ax35a3b6qr417yt8cd6n86uiw3d4whty3","type":"tcp"}}}"#
        );
    }

    #[test]
    fn deserialize_detailed_peers() {
        let json = r#"{"peers":{"[::ffff:172.17.0.1]:7075":{"protocol_version":18,"node_id":"node_1y7j5rdqhg99uyab1145gu3yur1ax35a3b6qr417yt8cd6n86uiw3d4whty3","type":"tcp"}}}"#;
        let peers: PeersDto = serde_json::from_str(json).unwrap();

        match peers.peers {
            PeerData::Detailed(map) => {
                assert_eq!(map.len(), 1);
                let peer_info = map.get("[::ffff:172.17.0.1]:7075").unwrap();
                match peer_info {
                    PeerInfo::Detailed {
                        protocol_version,
                        node_id,
                        connection_type,
                    } => {
                        assert_eq!(protocol_version, &18);
                        assert_eq!(
                            node_id,
                            &Account::decode_account(
                                "nano_1y7j5rdqhg99uyab1145gu3yur1ax35a3b6qr417yt8cd6n86uiw3d4whty3"
                            )
                            .unwrap()
                        );
                        assert_eq!(connection_type, "tcp");
                    }
                    PeerInfo::Simple(_) => panic!("Expected Detailed, got Simple"),
                }
            }
            PeerData::Simple(_) => panic!("Expected Detailed, got Simple"),
        }
    }
}
