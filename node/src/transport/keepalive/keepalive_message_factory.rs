use rsnano_core::utils::{Peer, NULL_ENDPOINT};
use rsnano_messages::{Keepalive, Message};
use rsnano_network::NetworkInfo;
use std::{
    net::{Ipv6Addr, SocketAddrV6},
    sync::{Arc, RwLock},
};

#[derive(Clone)]
pub struct KeepaliveMessageFactory {
    network: Arc<RwLock<NetworkInfo>>,
    external_addr: Peer,
}

impl KeepaliveMessageFactory {
    pub fn new(network: Arc<RwLock<NetworkInfo>>, external_addr: Peer) -> Self {
        Self {
            network,
            external_addr,
        }
    }

    pub fn create_keepalive_self(&self) -> Message {
        let mut result = Keepalive::default();
        let network = self.network.read().unwrap();
        network.random_fill_realtime(&mut result.peers);
        // We will clobber values in index 0 and 1 and if there are only 2 nodes in the system, these are the only positions occupied
        // Move these items to index 2 and 3 so they propagate
        result.peers[2] = result.peers[0];
        result.peers[3] = result.peers[1];
        // Replace part of message with node external address or listening port
        result.peers[1] = SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0); // For node v19 (response channels)
        if self.external_addr.address != Ipv6Addr::UNSPECIFIED.to_string()
            && self.external_addr.port != 0
        {
            result.peers[0] = SocketAddrV6::new(
                self.external_addr.address.parse().unwrap(),
                self.external_addr.port,
                0,
                0,
            );
        } else {
            // TODO Read external address from port_mapping!
            //let external_address  node.port_mapping.external_address ());
            let external_address = SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0);
            if !external_address.ip().is_unspecified() {
                result.peers[0] =
                    SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, network.listening_port(), 0, 0);
                result.peers[1] = external_address;
            } else {
                result.peers[0] =
                    SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, network.listening_port(), 0, 0);
            }
        }
        Message::Keepalive(result)
    }

    pub fn create_keepalive(&self) -> Message {
        let mut peers = [NULL_ENDPOINT; 8];
        self.network
            .read()
            .unwrap()
            .random_fill_realtime(&mut peers);

        Message::Keepalive(Keepalive { peers })
    }
}
