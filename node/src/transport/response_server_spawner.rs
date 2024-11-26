use super::{
    InboundMessageQueue, LatestKeepalives, MessagePublisher, NetworkFilter, ResponseServer,
    ResponseServerExt, SynCookies,
};
use crate::{
    block_processing::BlockProcessor,
    bootstrap::{BootstrapInitiator, BootstrapInitiatorConfig},
    config::NodeFlags,
    stats::Stats,
    utils::{ThreadPool, ThreadPoolImpl},
    NetworkParams,
};
use rsnano_core::{KeyPair, Networks};
use rsnano_ledger::Ledger;
use rsnano_network::{
    Channel, ChannelDirection, Network, NetworkInfo, NullNetworkObserver, ResponseServerSpawner,
};
use rsnano_nullable_clock::SteadyClock;
use std::sync::{Arc, Mutex, RwLock};

pub struct NanoResponseServerSpawner {
    pub(crate) tokio: tokio::runtime::Handle,
    pub(crate) stats: Arc<Stats>,
    pub(crate) node_id: KeyPair,
    pub(crate) ledger: Arc<Ledger>,
    pub(crate) workers: Arc<dyn ThreadPool>,
    pub(crate) block_processor: Arc<BlockProcessor>,
    pub(crate) bootstrap_initiator: Arc<BootstrapInitiator>,
    pub(crate) network: Arc<RwLock<NetworkInfo>>,
    pub(crate) publish_filter: Arc<NetworkFilter>,
    pub(crate) inbound_queue: Arc<InboundMessageQueue>,
    pub(crate) node_flags: NodeFlags,
    pub(crate) network_params: NetworkParams,
    pub(crate) syn_cookies: Arc<SynCookies>,
    pub(crate) latest_keepalives: Arc<Mutex<LatestKeepalives>>,
}

impl NanoResponseServerSpawner {
    #[allow(dead_code)]
    pub(crate) fn new_null(tokio: tokio::runtime::Handle) -> Self {
        let ledger = Arc::new(Ledger::new_null());
        let flags = NodeFlags::default();
        let network = Arc::new(Network::new_null(tokio.clone()));
        let publish_filter = Arc::new(NetworkFilter::default());
        let network_info = Arc::new(RwLock::new(NetworkInfo::new_test_instance()));
        let workers = Arc::new(ThreadPoolImpl::new_test_instance());
        let network_params = NetworkParams::new(Networks::NanoDevNetwork);
        let stats = Arc::new(Stats::default());
        let block_processor = Arc::new(BlockProcessor::new_test_instance(ledger.clone()));
        let clock = Arc::new(SteadyClock::new_null());
        Self {
            tokio: tokio.clone(),
            stats: stats.clone(),
            node_id: KeyPair::from(42),
            ledger: ledger.clone(),
            workers: Arc::new(ThreadPoolImpl::new_test_instance()),
            block_processor: block_processor.clone(),
            bootstrap_initiator: Arc::new(BootstrapInitiator::new(
                BootstrapInitiatorConfig::default_for(Networks::NanoDevNetwork),
                flags.clone(),
                network.clone(),
                network_info.clone(),
                Arc::new(NullNetworkObserver::new()),
                tokio.clone(),
                workers,
                network_params.clone(),
                stats,
                block_processor,
                ledger,
                MessagePublisher::new_null(tokio.clone()),
                clock,
            )),
            network: network_info,
            inbound_queue: Arc::new(InboundMessageQueue::default()),
            node_flags: flags,
            network_params,
            syn_cookies: Arc::new(SynCookies::new(1)),
            latest_keepalives: Arc::new(Mutex::new(LatestKeepalives::default())),
            publish_filter,
        }
    }

    pub(crate) fn spawn_outbound(&self, channel: Arc<Channel>) {
        let response_server = self.spawn_response_server(channel);
        self.tokio.spawn(async move {
            response_server.initiate_handshake().await;
        });
    }

    fn spawn_response_server(&self, channel: Arc<Channel>) -> Arc<ResponseServer> {
        let server = Arc::new(ResponseServer::new(
            self.network.clone(),
            self.inbound_queue.clone(),
            channel,
            self.publish_filter.clone(),
            Arc::new(self.network_params.clone()),
            Arc::clone(&self.stats),
            true,
            self.syn_cookies.clone(),
            self.node_id.clone(),
            self.tokio.clone(),
            self.ledger.clone(),
            self.workers.clone(),
            self.block_processor.clone(),
            self.bootstrap_initiator.clone(),
            self.node_flags.clone(),
            self.latest_keepalives.clone(),
        ));

        let server_l = server.clone();
        self.tokio.spawn(async move { server_l.run().await });

        server
    }
}

impl ResponseServerSpawner for NanoResponseServerSpawner {
    fn spawn(&self, channel: Arc<Channel>) {
        match channel.info.direction() {
            ChannelDirection::Inbound => {
                self.spawn_response_server(channel);
            }
            ChannelDirection::Outbound => self.spawn_outbound(channel),
        }
    }
}
