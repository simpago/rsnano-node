use super::{ChannelDirection, Network, ResponseServerFactory, SocketBuilder, TcpConfig};
use crate::{
    config::NodeConfig,
    stats::{DetailType, Direction, SocketStats, StatType, Stats},
    transport::TcpStream,
    utils::AsyncRuntime,
    NetworkParams,
};
use rsnano_core::utils::{OutputListenerMt, OutputTrackerMt};
use std::{net::SocketAddrV6, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use tracing::debug;

/// Establishes a network connection to a given peer
pub struct PeerConnector {
    config: TcpConfig,
    node_config: NodeConfig,
    network: Arc<Network>,
    stats: Arc<Stats>,
    runtime: Arc<AsyncRuntime>,
    network_params: NetworkParams,
    cancel_token: CancellationToken,
    response_server_factory: Arc<ResponseServerFactory>,
    connect_listener: OutputListenerMt<SocketAddrV6>,
}

impl PeerConnector {
    pub(crate) fn new(
        config: TcpConfig,
        node_config: NodeConfig,
        network: Arc<Network>,
        stats: Arc<Stats>,
        runtime: Arc<AsyncRuntime>,
        network_params: NetworkParams,
        response_server_factory: Arc<ResponseServerFactory>,
    ) -> Self {
        Self {
            config,
            node_config,
            network,
            stats,
            runtime,
            network_params,
            cancel_token: CancellationToken::new(),
            response_server_factory,
            connect_listener: OutputListenerMt::new(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn new_null() -> Self {
        Self {
            config: Default::default(),
            node_config: NodeConfig::new_test_instance(),
            network: Arc::new(Network::new_null()),
            stats: Arc::new(Default::default()),
            runtime: Arc::new(Default::default()),
            network_params: NetworkParams::new(rsnano_core::Networks::NanoDevNetwork),
            cancel_token: CancellationToken::new(),
            response_server_factory: Arc::new(ResponseServerFactory::new_null()),
            connect_listener: OutputListenerMt::new(),
        }
    }

    pub fn track_connections(&self) -> Arc<OutputTrackerMt<SocketAddrV6>> {
        self.connect_listener.track()
    }

    pub fn stop(&self) {
        self.cancel_token.cancel();
    }

    async fn connect_impl(&self, endpoint: SocketAddrV6) -> anyhow::Result<()> {
        let raw_listener = tokio::net::TcpSocket::new_v6()?;
        let raw_stream = raw_listener.connect(endpoint.into()).await?;
        let raw_stream = TcpStream::new(raw_stream);

        let socket_stats = Arc::new(SocketStats::new(Arc::clone(&self.stats)));
        let socket = SocketBuilder::new(ChannelDirection::Outbound, self.runtime.clone())
            .default_timeout(Duration::from_secs(
                self.node_config.tcp_io_timeout_s as u64,
            ))
            .silent_connection_tolerance_time(Duration::from_secs(
                self.network_params
                    .network
                    .silent_connection_tolerance_time_s as u64,
            ))
            .idle_timeout(self.network_params.network.idle_timeout)
            .observer(socket_stats)
            .finish(raw_stream);

        let response_server = self
            .response_server_factory
            .create_response_server(socket.clone());

        self.network
            .add(&socket, &response_server, ChannelDirection::Outbound)
            .await
    }
}

pub trait PeerConnectorExt {
    /// Establish a network connection to the given peer
    fn connect_to(&self, peer: SocketAddrV6);
}

impl PeerConnectorExt for Arc<PeerConnector> {
    fn connect_to(&self, peer: SocketAddrV6) {
        self.connect_listener.emit(peer);

        if self.cancel_token.is_cancelled() {
            return;
        }

        if !self.network.track_connection_attempt(&peer) {
            return;
        }

        self.stats.inc(StatType::Network, DetailType::MergePeer);

        let self_l = Arc::clone(self);
        self.runtime.tokio.spawn(async move {
            tokio::select! {
                result =  self_l.connect_impl(peer) =>{
                    if let Err(e) = result {
                        self_l.stats.inc_dir(
                            StatType::TcpListener,
                            DetailType::ConnectError,
                            Direction::Out,
                        );
                        debug!("Error connecting to: {} ({:?})", peer, e);
                    }

                },
                _ = tokio::time::sleep(self_l.config.connect_timeout) =>{
                    self_l.stats
                        .inc(StatType::TcpListener, DetailType::AttemptTimeout);
                    debug!(
                        "Connection attempt timed out: {}",
                        peer,
                    );

                }
                _ = self_l.cancel_token.cancelled() =>{
                    debug!(
                        "Connection attempt cancelled: {}",
                        peer,
                    );

                }
            }

            self_l.network.remove_attempt(&peer);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsnano_core::utils::TEST_ENDPOINT_1;

    #[test]
    fn track_connections() {
        let peer_connector = Arc::new(PeerConnector::new_null());
        let connect_tracker = peer_connector.track_connections();

        peer_connector.connect_to(TEST_ENDPOINT_1);

        assert_eq!(connect_tracker.output(), vec![TEST_ENDPOINT_1]);
    }
}
