use std::net::SocketAddrV6;
use rsnano_core::{Account, Amount};
use serde::{Deserialize, Serialize};
use crate::RpcCommand;

impl RpcCommand {
    pub fn confirmation_quorum(peer_details: Option<bool>) -> Self {
        Self::ConfirmationQuorum(ConfirmationQuorumArgs::new(peer_details))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ConfirmationQuorumArgs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_details: Option<bool>,
}

impl ConfirmationQuorumArgs {
    pub fn new(peer_details: Option<bool>) -> Self {
        Self { peer_details }
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ConfirmationQuorumDto {
    pub quorum_delta: Amount,
    pub online_weight_quorum_percent: u8,
    pub online_weight_minimum: Amount,
    pub online_stake_total: Amount,
    pub peers_stake_total: Amount,
    pub trended_stake_total: Amount,
    pub peers: Option<Vec<PeerDetailsDto>>,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct PeerDetailsDto {
    pub account: Account,
    pub ip: SocketAddrV6,
    pub weight: Amount,
}