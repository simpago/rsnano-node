use super::{InboundMessageQueue, LatestKeepalives, ResponseServer, SynCookies};
use crate::{stats::Stats, NetworkParams};
use rsnano_core::{Networks, PrivateKey};
use rsnano_messages::NetworkFilter;
use rsnano_network::{Network, ResponseServerSpawner, TcpChannelAdapter};
use std::sync::{Arc, Mutex, RwLock};

pub struct NanoResponseServerSpawner {
    pub(crate) tokio: tokio::runtime::Handle,
    pub(crate) stats: Arc<Stats>,
    pub(crate) node_id: PrivateKey,
    pub(crate) network: Arc<RwLock<Network>>,
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
        let network = Arc::new(RwLock::new(Network::new_test_instance()));
        let network_params = NetworkParams::new(Networks::NanoDevNetwork);
        let stats = Arc::new(Stats::default());
        Self {
            tokio: tokio.clone(),
            stats: stats.clone(),
            node_id: PrivateKey::from(42),
            network,
            inbound_queue: Arc::new(InboundMessageQueue::default()),
            network_params,
            syn_cookies: Arc::new(SynCookies::new(1)),
            latest_keepalives: Arc::new(Mutex::new(LatestKeepalives::default())),
            network_filter,
        }
    }
}

impl ResponseServerSpawner for NanoResponseServerSpawner {
    fn spawn(&self, channel_adapter: Arc<TcpChannelAdapter>) {
        let server = Arc::new(ResponseServer::new(
            self.network.clone(),
            self.inbound_queue.clone(),
            channel_adapter,
            self.network_filter.clone(),
            Arc::new(self.network_params.clone()),
            Arc::clone(&self.stats),
            self.syn_cookies.clone(),
            self.node_id.clone(),
            self.latest_keepalives.clone(),
        ));

        self.tokio.spawn(async move { server.run().await });
    }
}
