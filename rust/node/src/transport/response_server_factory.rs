use super::{
    InboundMessageQueue, LatestKeepalives, MessagePublisher, Network, NetworkFilter,
    ResponseServer, ResponseServerExt, SynCookies,
};
use crate::{
    block_processing::BlockProcessor,
    bootstrap::{BootstrapInitiator, BootstrapInitiatorConfig},
    config::NodeFlags,
    stats::Stats,
    utils::{AsyncRuntime, ThreadPool, ThreadPoolImpl},
    NetworkParams,
};
use rsnano_core::KeyPair;
use rsnano_ledger::Ledger;
use rsnano_network::{Channel, NetworkInfo, NullNetworkObserver};
use rsnano_nullable_clock::SteadyClock;
use std::sync::{Arc, Mutex, RwLock};

pub(crate) struct ResponseServerFactory {
    pub(crate) runtime: Arc<AsyncRuntime>,
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

impl ResponseServerFactory {
    #[allow(dead_code)]
    pub(crate) fn new_null() -> Self {
        let ledger = Arc::new(Ledger::new_null());
        let flags = NodeFlags::default();
        let network = Arc::new(Network::new_null());
        let publish_filter = Arc::new(NetworkFilter::default());
        let network_info = Arc::new(RwLock::new(NetworkInfo::new_test_instance()));
        let runtime = Arc::new(AsyncRuntime::default());
        let workers = Arc::new(ThreadPoolImpl::new_test_instance());
        let network_params = NetworkParams::new(rsnano_core::Networks::NanoDevNetwork);
        let stats = Arc::new(Stats::default());
        let block_processor = Arc::new(BlockProcessor::new_test_instance(ledger.clone()));
        let clock = Arc::new(SteadyClock::new_null());
        Self {
            runtime: runtime.clone(),
            stats: stats.clone(),
            node_id: KeyPair::from(42),
            ledger: ledger.clone(),
            workers: Arc::new(ThreadPoolImpl::new_test_instance()),
            block_processor: block_processor.clone(),
            bootstrap_initiator: Arc::new(BootstrapInitiator::new(
                BootstrapInitiatorConfig::default(),
                flags.clone(),
                network.clone(),
                network_info.clone(),
                Arc::new(NullNetworkObserver::new()),
                runtime,
                workers,
                network_params.clone(),
                stats,
                block_processor,
                None,
                ledger,
                MessagePublisher::new_null(),
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

    pub(crate) fn start_response_server(&self, channel: Arc<Channel>) -> Arc<ResponseServer> {
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
            self.runtime.clone(),
            self.ledger.clone(),
            self.workers.clone(),
            self.block_processor.clone(),
            self.bootstrap_initiator.clone(),
            self.node_flags.clone(),
            self.latest_keepalives.clone(),
        ));

        let server_l = server.clone();
        tokio::spawn(async move { server_l.run().await });

        server
    }
}
