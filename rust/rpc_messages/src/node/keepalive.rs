use crate::RpcCommand;
use serde::{Deserialize, Serialize};
use std::net::Ipv6Addr;

impl RpcCommand {
    pub fn keepalive(address: Ipv6Addr, port: u16) -> Self {
        Self::Keepalive(KeepaliveArgs { address, port })
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct KeepaliveArgs {
    pub address: Ipv6Addr,
    pub port: u16,
}
