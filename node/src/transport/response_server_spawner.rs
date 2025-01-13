use super::{
    InboundMessageQueue, LatestKeepalives, NetworkFilter, ResponseServer, ResponseServerExt,
    SynCookies,
};
use crate::{stats::Stats, NetworkParams};
use rsnano_core::{Networks, PrivateKey};
use rsnano_network::{ChannelAdapter, ChannelDirection, NetworkInfo, ResponseServerSpawner};
use std::sync::{Arc, Mutex, RwLock};

pub struct NanoResponseServerSpawner {
    pub(crate) tokio: tokio::runtime::Handle,
    pub(crate) stats: Arc<Stats>,
    pub(crate) node_id: PrivateKey,
    pub(crate) network: Arc<RwLock<NetworkInfo>>,
    pub(crate) network_filter: Arc<NetworkFilter>,
    pub(crate) inbound_queue: Arc<InboundMessageQueue>,
    pub(crate) network_params: NetworkParams,
    pub(crate) syn_cookies: Arc<SynCookies>,
    pub(crate) latest_keepalives: Arc<Mutex<LatestKeepalives>>,
}

impl NanoResponseServerSpawner {
    #[allow(dead_code)]
    pub(crate) fn new_null(tokio: tokio::runtime::Handle) -> Self {
        let network_filter = Arc::new(NetworkFilter::default());
        let network_info = Arc::new(RwLock::new(NetworkInfo::new_test_instance()));
        let network_params = NetworkParams::new(Networks::NanoDevNetwork);
        let stats = Arc::new(Stats::default());
        Self {
            tokio: tokio.clone(),
            stats: stats.clone(),
            node_id: PrivateKey::from(42),
            network: network_info,
            inbound_queue: Arc::new(InboundMessageQueue::default()),
            network_params,
            syn_cookies: Arc::new(SynCookies::new(1)),
            latest_keepalives: Arc::new(Mutex::new(LatestKeepalives::default())),
            network_filter,
        }
    }

    pub(crate) fn spawn_outbound(&self, channel_adapter: Arc<ChannelAdapter>) {
        let response_server = self.spawn_response_server(channel_adapter);
        self.tokio.spawn(async move {
            response_server.initiate_handshake().await;
        });
    }

    fn spawn_response_server(&self, channel_adapter: Arc<ChannelAdapter>) -> Arc<ResponseServer> {
        let server = Arc::new(ResponseServer::new(
            self.network.clone(),
            self.inbound_queue.clone(),
            channel_adapter,
            self.network_filter.clone(),
            Arc::new(self.network_params.clone()),
            Arc::clone(&self.stats),
            true,
            self.syn_cookies.clone(),
            self.node_id.clone(),
            self.latest_keepalives.clone(),
        ));

        let server_l = server.clone();
        self.tokio.spawn(async move { server_l.run().await });

        server
    }
}

impl ResponseServerSpawner for NanoResponseServerSpawner {
    fn spawn(&self, channel_adapter: Arc<ChannelAdapter>) {
        match channel_adapter.info.direction() {
            ChannelDirection::Inbound => {
                self.spawn_response_server(channel_adapter);
            }
            ChannelDirection::Outbound => self.spawn_outbound(channel_adapter),
        }
    }
}
