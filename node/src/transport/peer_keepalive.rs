use rsnano_core::utils::{Peer, NULL_ENDPOINT};
use rsnano_messages::{Keepalive, Message};
use rsnano_network::{
    utils::into_ipv6_socket_address, ChannelId, DropPolicy, NetworkInfo, PeerConnector, TrafficType,
};
use rsnano_output_tracker::{OutputListenerMt, OutputTrackerMt};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex, RwLock},
};
use tracing::error;

use super::MessagePublisher;

/// Connects to a peer if we don't have a connection
/// or it sends a keepalive message if we are already connected
pub struct PeerKeeplive {
    keepalive_listener: OutputListenerMt<Peer>,
    network: Arc<RwLock<NetworkInfo>>,
    peer_connector: Arc<PeerConnector>,
    message_publisher: Mutex<MessagePublisher>,
}

impl PeerKeeplive {
    pub fn new(
        network: Arc<RwLock<NetworkInfo>>,
        peer_connector: Arc<PeerConnector>,
        message_publisher: MessagePublisher,
    ) -> Self {
        Self {
            keepalive_listener: OutputListenerMt::new(),
            network,
            peer_connector,
            message_publisher: Mutex::new(message_publisher),
        }
    }

    pub fn track_keepalives(&self) -> Arc<OutputTrackerMt<Peer>> {
        self.keepalive_listener.track()
    }

    pub async fn keepalive_or_connect(&self, address: String, port: u16) {
        self.keepalive_listener
            .emit(Peer::new(address.clone(), port));
        match tokio::net::lookup_host((address.as_str(), port)).await {
            Ok(addresses) => {
                for addr in addresses {
                    self.keepalive_or_connect_socket(addr);
                }
            }
            Err(e) => {
                error!(
                    "Error resolving address for keepalive: {}:{} ({})",
                    address, port, e
                )
            }
        }
    }

    fn keepalive_or_connect_socket(&self, peer: SocketAddr) {
        let peer_v6 = into_ipv6_socket_address(peer);

        let channel_id = self
            .network
            .read()
            .unwrap()
            .find_realtime_channel_by_peering_addr(&peer_v6);

        match channel_id {
            Some(channel_id) => {
                self.try_send_keepalive(channel_id);
            }
            None => {
                self.peer_connector.connect_to(peer_v6);
            }
        }
    }

    fn try_send_keepalive(&self, channel_id: ChannelId) {
        let keepalive = self.create_keepalive_message();

        self.message_publisher.lock().unwrap().try_send(
            channel_id,
            &keepalive,
            DropPolicy::CanDrop,
            TrafficType::Generic,
        );
    }

    fn create_keepalive_message(&self) -> Message {
        let mut peers = [NULL_ENDPOINT; 8];
        self.network
            .read()
            .unwrap()
            .random_fill_realtime(&mut peers);

        Message::Keepalive(Keepalive { peers })
    }
}

/// Connect to preconfigured peers or send keepalive messages
/// if we are already connected
pub(crate) struct PreconfiguredPeersKeepalive {
    peers: Vec<Peer>,
    pub keepalive: Arc<PeerKeeplive>,
}

impl PreconfiguredPeersKeepalive {
    pub(crate) fn new(peers: Vec<Peer>, keepalive: Arc<PeerKeeplive>) -> Self {
        Self { peers, keepalive }
    }

    pub async fn keepalive(&self) {
        for peer in &self.peers {
            self.keepalive
                .keepalive_or_connect(peer.address.clone(), peer.port)
                .await;
        }
    }
}
