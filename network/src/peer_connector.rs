use crate::{
    ChannelDirection, ChannelMode, Network, NetworkObserver, NullNetworkObserver,
    NullResponseServerSpawner, ResponseServerSpawner,
};
use rsnano_nullable_clock::SteadyClock;
use rsnano_nullable_tcp::TcpStream;
use rsnano_output_tracker::{OutputListenerMt, OutputTrackerMt};
use std::{net::SocketAddrV6, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

/// Establishes a network connection to a given peer
pub struct PeerConnector {
    connect_timeout: Duration,
    network: Arc<Network>,
    network_observer: Arc<dyn NetworkObserver>,
    tokio: tokio::runtime::Handle,
    cancel_token: CancellationToken,
    response_server_spawner: Arc<dyn ResponseServerSpawner>,
    connect_listener: OutputListenerMt<SocketAddrV6>,
    clock: Arc<SteadyClock>,
}

impl PeerConnector {
    const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

    pub fn new(
        connect_timeout: Duration,
        network: Arc<Network>,
        network_observer: Arc<dyn NetworkObserver>,
        tokio: tokio::runtime::Handle,
        response_server_spawner: Arc<dyn ResponseServerSpawner>,
        clock: Arc<SteadyClock>,
    ) -> Self {
        Self {
            connect_timeout,
            network,
            network_observer,
            tokio,
            cancel_token: CancellationToken::new(),
            response_server_spawner,
            connect_listener: OutputListenerMt::new(),
            clock,
        }
    }

    pub fn new_null(tokio: tokio::runtime::Handle) -> Self {
        Self {
            connect_timeout: Self::DEFAULT_TIMEOUT,
            network: Arc::new(Network::new_null(tokio.clone())),
            network_observer: Arc::new(NullNetworkObserver::new()),
            tokio: tokio.clone(),
            cancel_token: CancellationToken::new(),
            response_server_spawner: Arc::new(NullResponseServerSpawner::new()),
            connect_listener: OutputListenerMt::new(),
            clock: Arc::new(SteadyClock::new_null()),
        }
    }

    pub fn track_connections(&self) -> Arc<OutputTrackerMt<SocketAddrV6>> {
        self.connect_listener.track()
    }

    /// Establish a network connection to the given peer
    pub fn connect_to(&self, peer: SocketAddrV6) -> bool {
        self.connect_listener.emit(peer);

        if self.cancel_token.is_cancelled() {
            return false;
        }

        {
            let mut network = self.network.info.write().unwrap();

            if let Err(e) =
                network.add_outbound_attempt(peer, ChannelMode::Realtime, self.clock.now())
            {
                drop(network);
                self.network_observer
                    .error(e, &peer, ChannelDirection::Outbound);

                return false;
            }

            if let Err(e) = network.validate_new_connection(
                &peer,
                ChannelDirection::Outbound,
                ChannelMode::Realtime,
                self.clock.now(),
            ) {
                network.remove_attempt(&peer);
                drop(network);
                self.network_observer
                    .error(e, &peer, ChannelDirection::Outbound);
                return false;
            }
        }

        self.network_observer.connection_attempt(&peer);
        self.network_observer.merge_peer();

        let network_l = self.network.clone();
        let response_server_spawner_l = self.response_server_spawner.clone();
        let connect_timeout = self.connect_timeout;
        let cancel_token = self.cancel_token.clone();
        let observer = self.network_observer.clone();

        self.tokio.spawn(async move {
            tokio::select! {
                result =  connect_impl(peer, &network_l, &*response_server_spawner_l) =>{
                    if let Err(e) = result {
                        observer.connect_error(peer, e);
                    }

                },
                _ = tokio::time::sleep(connect_timeout) =>{
                    observer.attempt_timeout(peer);

                }
                _ = cancel_token.cancelled() =>{
                    observer.attempt_cancelled(peer);

                }
            }

            network_l.info.write().unwrap().remove_attempt(&peer);
        });

        true
    }

    pub fn stop(&self) {
        self.cancel_token.cancel();
    }
}

async fn connect_impl(
    peer: SocketAddrV6,
    network: &Network,
    response_server_spawner: &dyn ResponseServerSpawner,
) -> anyhow::Result<()> {
    let tcp_stream = connect_stream(peer).await?;

    let channel = network.add(
        tcp_stream,
        ChannelDirection::Outbound,
        ChannelMode::Realtime,
    )?;

    response_server_spawner.spawn(channel);
    Ok(())
}

async fn connect_stream(peer: SocketAddrV6) -> tokio::io::Result<TcpStream> {
    let socket = tokio::net::TcpSocket::new_v6()?;
    let tcp_stream = socket.connect(peer.into()).await?;
    Ok(TcpStream::new(tcp_stream))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsnano_core::utils::TEST_ENDPOINT_1;

    #[tokio::test]
    async fn track_connections() {
        let peer_connector = Arc::new(PeerConnector::new_null(tokio::runtime::Handle::current()));
        let connect_tracker = peer_connector.track_connections();

        peer_connector.connect_to(TEST_ENDPOINT_1);

        assert_eq!(connect_tracker.output(), vec![TEST_ENDPOINT_1]);
    }
}
