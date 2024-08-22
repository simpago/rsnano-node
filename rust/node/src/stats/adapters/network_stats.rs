use crate::stats::{DetailType, Direction, StatType, Stats};
use rsnano_network::{ChannelDirection, ChannelInfo, NetworkError, NetworkObserver};
use std::{net::SocketAddrV6, sync::Arc};
use tracing::debug;

#[derive(Clone)]
pub struct NetworkStats(Arc<Stats>);

impl NetworkStats {
    pub fn new(stats: Arc<Stats>) -> Self {
        Self(stats)
    }
}

impl NetworkObserver for NetworkStats {
    fn send_succeeded(&self, buf_size: usize) {
        self.0.add_dir_aggregate(
            StatType::TrafficTcp,
            DetailType::All,
            Direction::Out,
            buf_size as u64,
        );
    }

    fn send_failed(&self) {
        self.0
            .inc_dir(StatType::Tcp, DetailType::TcpWriteError, Direction::In);
    }

    fn channel_timed_out(&self, channel: &ChannelInfo) {
        self.0.inc_dir(
            StatType::Tcp,
            DetailType::TcpIoTimeoutDrop,
            if channel.direction() == ChannelDirection::Inbound {
                Direction::In
            } else {
                Direction::Out
            },
        );
        debug!(
            channel_id = %channel.channel_id(), 
            remote_addr = ?channel.peer_addr(), 
            mode = ?channel.mode(), 
            direction = ?channel.direction(), 
            "Closing channel due to timeout");
    }

    fn read_succeeded(&self, count: usize) {
        self.0.add_dir(
            StatType::TrafficTcp,
            DetailType::All,
            Direction::In,
            count as u64,
        );
    }

    fn read_failed(&self) {
        self.0.inc_dir(
            StatType::Tcp,
            DetailType::TcpReadError,
            Direction::In,
        );
    }

    fn connection_attempt(&self, peer: &SocketAddrV6) {
        self.0.inc_dir(
            StatType::TcpListener,
            DetailType::ConnectInitiate,
            Direction::Out,
        );
        debug!(?peer, "Initiate outgoing connection");
    }

    fn accepted(&self, peer: &SocketAddrV6, direction: ChannelDirection) {
        if direction == ChannelDirection::Outbound {
            self.0.inc_dir(
                StatType::TcpListener,
                DetailType::ConnectSuccess,
                direction.into(),
            );
        } else {
            self.0.inc_dir(
                StatType::TcpListener,
                DetailType::AcceptSuccess,
                direction.into(),
            );
        }
        debug!(%peer, ?direction, "New channel added");
    }

    fn error(&self, error: NetworkError, peer: &SocketAddrV6, direction: ChannelDirection) {
        match direction {
            ChannelDirection::Inbound => {
                self.0.inc_dir(
                    StatType::TcpListener,
                    DetailType::AcceptRejected,
                    Direction::In,
                );
            }
            ChannelDirection::Outbound => {
                self.0.inc_dir(
                    StatType::TcpListener,
                    DetailType::ConnectRejected,
                    Direction::Out,
                );
            }
        }

        match error {
            NetworkError::MaxConnections => {
                self.0.inc_dir(
                    StatType::TcpListenerRejected,
                    DetailType::MaxAttempts,
                    direction.into(),
                );
                debug!(
                    %peer,
                    ?direction,
                    "Max connections reached, unable to make new connection",
                );
            }
            NetworkError::PeerExcluded => {
                self.0.inc_dir(
                    StatType::TcpListenerRejected,
                    DetailType::Excluded,
                    direction.into(),
                );
                debug!(
                    %peer,
                    ?direction,
                    "Peer excluded, unable to make new connection",
                );
            }
            NetworkError::MaxConnectionsPerSubnetwork => {
                self.0.inc_dir(
                    StatType::TcpListenerRejected,
                    DetailType::MaxPerSubnetwork,
                    direction.into(),
                );
                self.0.inc_dir(
                    StatType::Tcp,
                    DetailType::MaxPerSubnetwork,
                    direction.into(),
                );
                debug!(
                    %peer,
                    ?direction,
                    "Max connections per subnetwork reached, unable to open new connection",
                );
            }
            NetworkError::MaxConnectionsPerIp => {
                self.0.inc_dir(
                    StatType::TcpListenerRejected,
                    DetailType::MaxPerIp,
                    direction.into(),
                );
                self.0
                    .inc_dir(StatType::Tcp, DetailType::MaxPerIp, direction.into());
                debug!(
                    %peer,
                    ?direction,
                    "Max connections per IP reached, unable to open new connection");
            }
            NetworkError::InvalidIp => {
                self.0.inc_dir(
                    StatType::TcpListenerRejected,
                    DetailType::NotAPeer,
                    direction.into(),
                );
                debug!(
                    %peer,
                    ?direction,
                    "Invalid IP, unable to open new connection");
            }
            NetworkError::DuplicateConnection => {
                self.0.inc_dir(
                    StatType::TcpListenerRejected,
                    DetailType::Duplicate,
                    direction.into(),
                );
                debug!(
                    %peer,
                    ?direction,
                    "Already connected to that peer, unable to open new connection");
            }
        }
    }
}

impl From<ChannelDirection> for Direction {
    fn from(value: ChannelDirection) -> Self {
        match value {
            ChannelDirection::Inbound => Direction::In,
            ChannelDirection::Outbound => Direction::Out,
        }
    }
}
