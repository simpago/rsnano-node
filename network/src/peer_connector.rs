use crate::{
    ChannelDirection, NetworkObserver, NullNetworkObserver, NullResponseServerSpawner,
    ResponseServerSpawner, TcpNetworkAdapter,
};
use rsnano_nullable_tcp::TcpStream;
use rsnano_output_tracker::{OutputListenerMt, OutputTrackerMt};
use std::{net::SocketAddrV6, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

/// Establishes a network connection to a given peer
pub struct PeerConnector {
    connect_timeout: Duration,
    network_adapter: Arc<TcpNetworkAdapter>,
    network_observer: Arc<dyn NetworkObserver>,
    tokio: tokio::runtime::Handle,
    cancel_token: CancellationToken,
    response_server_spawner: Arc<dyn ResponseServerSpawner>,
    connect_listener: OutputListenerMt<SocketAddrV6>,
}

impl PeerConnector {
    const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

    pub fn new(
        connect_timeout: Duration,
        network_adapter: Arc<TcpNetworkAdapter>,
        network_observer: Arc<dyn NetworkObserver>,
        tokio: tokio::runtime::Handle,
        response_server_spawner: Arc<dyn ResponseServerSpawner>,
    ) -> Self {
        Self {
            connect_timeout,
            network_adapter,
            network_observer,
            tokio,
            cancel_token: CancellationToken::new(),
            response_server_spawner,
            connect_listener: OutputListenerMt::new(),
        }
    }

    pub fn new_null(tokio: tokio::runtime::Handle) -> Self {
        Self {
            connect_timeout: Self::DEFAULT_TIMEOUT,
            network_adapter: Arc::new(TcpNetworkAdapter::new_null(tokio.clone())),
            network_observer: Arc::new(NullNetworkObserver::new()),
            tokio: tokio.clone(),
            cancel_token: CancellationToken::new(),
            response_server_spawner: Arc::new(NullResponseServerSpawner::new()),
            connect_listener: OutputListenerMt::new(),
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

        let added = self.network_adapter.add_outbound_attempt(peer);

        if !added {
            return false;
        }

        let network_l = self.network_adapter.clone();
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

            network_l.remove_attempt(&peer);
        });

        true
    }

    pub fn stop(&self) {
        self.cancel_token.cancel();
    }
}

async fn connect_impl(
    peer: SocketAddrV6,
    network_adapter: &TcpNetworkAdapter,
    response_server_spawner: &dyn ResponseServerSpawner,
) -> anyhow::Result<()> {
    let tcp_stream = connect_stream(peer).await?;
    let channel = network_adapter.add(tcp_stream, ChannelDirection::Outbound)?;
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
