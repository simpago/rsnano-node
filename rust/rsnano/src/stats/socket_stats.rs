use std::{net::SocketAddr, sync::Arc};

use crate::{
    logger_mt::Logger,
    network::{EndpointType, SocketObserver},
    utils::ErrorCode,
};

use super::{DetailType, Direction, Stat, StatType};

pub struct SocketStats {
    stats: Arc<Stat>,
    logger: Arc<dyn Logger>,
    enable_timeout_logging: bool,
}

impl SocketStats {
    pub fn new(stats: Arc<Stat>, logger: Arc<dyn Logger>, enable_timeout_logging: bool) -> Self {
        Self {
            stats,
            logger,
            enable_timeout_logging,
        }
    }
}

impl SocketObserver for SocketStats {
    fn close_socket_failed(&self, ec: ErrorCode) {
        self.logger
            .try_log(&format!("Failed to close socket gracefully: {:?}", ec));
        let _ = self.stats.inc(
            StatType::Bootstrap,
            DetailType::ErrorSocketClose,
            Direction::In,
        );
    }

    fn disconnect_due_to_timeout(&self, endpoint: SocketAddr) {
        if self.enable_timeout_logging {
            self.logger
                .try_log(&format!("Disconnecting from {} due to timeout", endpoint));
        }
    }

    fn connect_error(&self) {
        let _ = self
            .stats
            .inc(StatType::Tcp, DetailType::TcpConnectError, Direction::In);
    }

    fn read_error(&self) {
        let _ = self
            .stats
            .inc(StatType::Tcp, DetailType::TcpReadError, Direction::In);
    }

    fn read_successful(&self, len: usize) {
        let _ = self.stats.add(
            StatType::TrafficTcp,
            DetailType::All,
            Direction::In,
            len as u64,
            false,
        );
    }

    fn write_error(&self) {
        let _ = self
            .stats
            .inc(StatType::Tcp, DetailType::TcpWriteError, Direction::In);
    }

    fn write_successful(&self, len: usize) {
        let _ = self.stats.add(
            StatType::TrafficTcp,
            DetailType::All,
            Direction::Out,
            len as u64,
            false,
        );
    }

    fn silent_connection_dropped(&self) {
        let _ = self.stats.inc(
            StatType::Tcp,
            DetailType::TcpSilentConnectionDrop,
            Direction::In,
        );
    }

    fn inactive_connection_dropped(&self, endpoint_type: EndpointType) {
        let _ = self.stats.inc(
            StatType::Tcp,
            DetailType::TcpIoTimeoutDrop,
            if endpoint_type == EndpointType::Server {
                Direction::In
            } else {
                Direction::Out
            },
        );
    }
}
