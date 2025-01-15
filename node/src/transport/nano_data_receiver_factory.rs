use super::{
    nano_data_receiver::NanoDataReceiver, DataReceiver, HandshakeProcess, InboundMessageQueue,
    LatestKeepalives, SynCookies,
};
use crate::{stats::Stats, NetworkParams};
use rsnano_core::PrivateKey;
use rsnano_messages::*;
use rsnano_network::{Channel, Network};
use std::sync::{Arc, Mutex, RwLock};

pub(crate) struct NanoDataReceiverFactory {
    network_params: Arc<NetworkParams>,
    stats: Arc<Stats>,
    network: Arc<RwLock<Network>>,
    inbound_queue: Arc<InboundMessageQueue>,
    network_filter: Arc<NetworkFilter>,
    syn_cookies: Arc<SynCookies>,
    node_id: PrivateKey,
    latest_keepalives: Arc<Mutex<LatestKeepalives>>,
}

impl NanoDataReceiverFactory {
    pub fn new(
        network: Arc<RwLock<Network>>,
        inbound_queue: Arc<InboundMessageQueue>,
        network_filter: Arc<NetworkFilter>,
        network_params: Arc<NetworkParams>,
        stats: Arc<Stats>,
        syn_cookies: Arc<SynCookies>,
        node_id: PrivateKey,
        latest_keepalives: Arc<Mutex<LatestKeepalives>>,
    ) -> Self {
        Self {
            network,
            inbound_queue,
            syn_cookies: syn_cookies.clone(),
            node_id: node_id.clone(),
            network_params,
            stats: stats.clone(),
            network_filter,
            latest_keepalives,
        }
    }

    pub fn create_data_receiver(&self, channel: Arc<Channel>) -> Box<dyn DataReceiver + Send> {
        let handshake_process = HandshakeProcess::new(
            self.network_params.ledger.genesis_block.hash(),
            self.node_id.clone(),
            self.syn_cookies.clone(),
            self.stats.clone(),
            self.network_params.network.protocol_info(),
        );

        let message_deserializer = MessageDeserializer::new(
            self.network_params.network.protocol_info(),
            self.network_filter.clone(),
            self.network_params.network.work.clone(),
        );

        Box::new(NanoDataReceiver::new(
            channel,
            self.network_params.clone(),
            handshake_process,
            message_deserializer,
            self.inbound_queue.clone(),
            self.latest_keepalives.clone(),
            self.stats.clone(),
            self.network.clone(),
        ))
    }
}
