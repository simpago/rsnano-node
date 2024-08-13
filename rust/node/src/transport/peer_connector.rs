use super::{AcceptResult, ChannelDirection, Network, ResponseServerFactory, TcpConfig};
use crate::{
    stats::{DetailType, Direction, StatType, Stats},
    transport::{ChannelMode, TcpStream},
    utils::AsyncRuntime,
};
use rsnano_core::utils::{OutputListenerMt, OutputTrackerMt};
use std::{net::SocketAddrV6, sync::Arc};
use tokio_util::sync::CancellationToken;
use tracing::debug;

/// Establishes a network connection to a given peer
pub struct PeerConnector {
    config: TcpConfig,
    network: Arc<Network>,
    stats: Arc<Stats>,
    runtime: Arc<AsyncRuntime>,
    cancel_token: CancellationToken,
    response_server_factory: Arc<ResponseServerFactory>,
    connect_listener: OutputListenerMt<SocketAddrV6>,
}

impl PeerConnector {
    pub(crate) fn new(
        config: TcpConfig,
        network: Arc<Network>,
        stats: Arc<Stats>,
        runtime: Arc<AsyncRuntime>,
        response_server_factory: Arc<ResponseServerFactory>,
    ) -> Self {
        Self {
            config,
            network,
            stats,
            runtime,
            cancel_token: CancellationToken::new(),
            response_server_factory,
            connect_listener: OutputListenerMt::new(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn new_null() -> Self {
        Self {
            config: Default::default(),
            network: Arc::new(Network::new_null()),
            stats: Arc::new(Default::default()),
            runtime: Arc::new(Default::default()),
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
        let channel = self
            .network
            .add(
                raw_stream,
                ChannelDirection::Outbound,
                ChannelMode::Realtime,
            )
            .await?;
        let response_server = self.response_server_factory.start_response_server(channel);
        response_server.initiate_handshake().await;
        Ok(())
    }
}

pub trait PeerConnectorExt {
    /// Establish a network connection to the given peer
    fn connect_to(&self, peer: SocketAddrV6) -> bool;
}

impl PeerConnectorExt for Arc<PeerConnector> {
    fn connect_to(&self, peer: SocketAddrV6) -> bool {
        self.connect_listener.emit(peer);

        if self.cancel_token.is_cancelled() {
            return false;
        }

        if !self.network.add_attempt(peer) {
            return false;
        }

        if self
            .network
            .can_add_connection(&peer, ChannelDirection::Outbound, ChannelMode::Realtime)
            != AcceptResult::Accepted
        {
            self.network.remove_attempt(&peer);
            return false;
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

        true
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
