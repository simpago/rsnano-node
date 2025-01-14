use super::SynCookies;
use crate::stats::{DetailType, Direction, StatType, Stats};
use rsnano_core::{utils::TEST_ENDPOINT_1, BlockHash, NodeId, PrivateKey};
use rsnano_messages::{
    Message, MessageSerializer, NodeIdHandshake, NodeIdHandshakeQuery, NodeIdHandshakeResponse,
    ProtocolInfo,
};
use rsnano_network::{Channel, DropPolicy, TrafficType};
use std::{
    net::SocketAddrV6,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tracing::{debug, warn};

pub enum HandshakeStatus {
    Abort,
    AbortOwnNodeId,
    Handshake,
    Realtime(NodeId),
    Bootstrap,
}

/// Responsible for performing a correct handshake when connecting to another node
pub(crate) struct HandshakeProcess {
    genesis_hash: BlockHash,
    node_id: PrivateKey,
    syn_cookies: Arc<SynCookies>,
    stats: Arc<Stats>,
    handshake_received: AtomicBool,
    peer_addr: SocketAddrV6,
    protocol: ProtocolInfo,
}

impl HandshakeProcess {
    pub(crate) fn new(
        genesis_hash: BlockHash,
        node_id: PrivateKey,
        syn_cookies: Arc<SynCookies>,
        stats: Arc<Stats>,
        peer_addr: SocketAddrV6,
        protocol: ProtocolInfo,
    ) -> Self {
        Self {
            genesis_hash,
            node_id,
            syn_cookies,
            stats,
            handshake_received: AtomicBool::new(false),
            peer_addr,
            protocol,
        }
    }

    #[allow(dead_code)]
    pub fn new_null() -> Self {
        Self {
            genesis_hash: BlockHash::from(1),
            node_id: PrivateKey::from(2),
            syn_cookies: Arc::new(SynCookies::new(1)),
            stats: Arc::new(Stats::default()),
            handshake_received: AtomicBool::new(false),
            peer_addr: TEST_ENDPOINT_1,
            protocol: ProtocolInfo::default(),
        }
    }

    pub(crate) async fn initiate_handshake(&self, channel: &Channel) -> Result<(), ()> {
        let endpoint = self.peer_addr;
        let query = self.prepare_query(&endpoint);
        if query.is_none() {
            warn!(
                "Could not create cookie for {:?}. Closing channel.",
                endpoint
            );
            return Err(());
        }
        let message = Message::NodeIdHandshake(NodeIdHandshake {
            query,
            response: None,
            is_v2: true,
        });

        debug!("Initiating handshake query ({})", endpoint);

        let mut serializer = MessageSerializer::new(self.protocol);
        let data = serializer.serialize(&message);

        match channel.send_buffer(data, TrafficType::Generic).await {
            Ok(()) => {
                self.stats
                    .inc_dir(StatType::TcpServer, DetailType::Handshake, Direction::Out);
                self.stats.inc_dir(
                    StatType::TcpServer,
                    DetailType::HandshakeInitiate,
                    Direction::Out,
                );

                Ok(())
            }
            Err(e) => {
                self.stats
                    .inc(StatType::TcpServer, DetailType::HandshakeNetworkError);
                debug!("Error sending handshake query: {:?} ({})", e, endpoint);

                // Stop invalid handshake
                Err(())
            }
        }
    }

    pub(crate) fn process_handshake(
        &self,
        message: &NodeIdHandshake,
        channel: &Channel,
    ) -> HandshakeStatus {
        if message.query.is_none() && message.response.is_none() {
            self.stats.inc_dir(
                StatType::TcpServer,
                DetailType::HandshakeError,
                Direction::In,
            );
            debug!("Invalid handshake message received ({})", self.peer_addr);
            return HandshakeStatus::Abort;
        }
        if message.query.is_some() && self.handshake_received.load(Ordering::SeqCst) {
            // Second handshake message should be a response only
            self.stats.inc_dir(
                StatType::TcpServer,
                DetailType::HandshakeError,
                Direction::In,
            );
            warn!("Detected multiple handshake queries ({})", self.peer_addr);
            return HandshakeStatus::Abort;
        }

        self.handshake_received.store(true, Ordering::SeqCst);

        self.stats.inc_dir(
            StatType::TcpServer,
            DetailType::NodeIdHandshake,
            Direction::In,
        );

        let log_type = match (message.query.is_some(), message.response.is_some()) {
            (true, true) => "query + response",
            (true, false) => "query",
            (false, true) => "response",
            (false, false) => "none",
        };
        debug!(
            "Handshake message received: {} ({})",
            log_type, self.peer_addr
        );

        if let Some(query) = message.query.clone() {
            // Send response + our own query
            if self.send_response(&query, message.is_v2, &channel).is_err() {
                // Stop invalid handshake
                return HandshakeStatus::Abort;
            }
            // Fall through and continue handshake
        }
        if let Some(response) = &message.response {
            match self.verify_response(response, &self.peer_addr) {
                Ok(()) => {
                    self.stats
                        .inc_dir(StatType::Handshake, DetailType::Ok, Direction::In);
                    return HandshakeStatus::Realtime(response.node_id); // Switch to realtime
                }
                Err(HandshakeResponseError::OwnNodeId) => {
                    warn!(
                        "This node tried to connect to itself. Closing channel ({})",
                        self.peer_addr
                    );
                    return HandshakeStatus::AbortOwnNodeId;
                }
                Err(e) => {
                    self.stats
                        .inc_dir(StatType::Handshake, e.into(), Direction::In);
                    self.stats.inc_dir(
                        StatType::TcpServer,
                        DetailType::HandshakeResponseInvalid,
                        Direction::In,
                    );
                    warn!(
                        "Invalid handshake response received ({}, {:?})",
                        self.peer_addr, e
                    );
                    return HandshakeStatus::Abort;
                }
            }
        }
        HandshakeStatus::Handshake // Handshake is in progress
    }

    fn send_response(
        &self,
        query: &NodeIdHandshakeQuery,
        v2: bool,
        channel: &Channel,
    ) -> anyhow::Result<()> {
        let response = self.prepare_response(query, v2);
        let own_query = self.prepare_query(&self.peer_addr);

        let handshake_response = Message::NodeIdHandshake(NodeIdHandshake {
            is_v2: own_query.is_some() || response.v2.is_some(),
            query: own_query,
            response: Some(response),
        });

        debug!("Responding to handshake ({})", self.peer_addr);

        let mut serializer = MessageSerializer::new(self.protocol);
        let buffer = serializer.serialize(&handshake_response);

        let enqueued =
            channel.try_send_buffer(buffer, DropPolicy::ShouldNotDrop, TrafficType::Generic);

        if enqueued {
            self.stats
                .inc_dir(StatType::TcpServer, DetailType::Handshake, Direction::Out);
            self.stats.inc_dir(
                StatType::TcpServer,
                DetailType::HandshakeResponse,
                Direction::Out,
            );
            Ok(())
        } else {
            self.stats.inc_dir(
                StatType::TcpServer,
                DetailType::HandshakeNetworkError,
                Direction::In,
            );
            warn!(peer = %self.peer_addr, "Error sending handshake response");
            Err(anyhow!("Could now enqueue handshake response"))
        }
    }

    fn verify_response(
        &self,
        response: &NodeIdHandshakeResponse,
        peer_addr: &SocketAddrV6,
    ) -> Result<(), HandshakeResponseError> {
        // Prevent connection with ourselves
        if response.node_id == self.node_id.public_key().into() {
            return Err(HandshakeResponseError::OwnNodeId);
        }

        // Prevent mismatched genesis
        if let Some(v2) = &response.v2 {
            if v2.genesis != self.genesis_hash {
                return Err(HandshakeResponseError::InvalidGenesis);
            }
        }

        let Some(cookie) = self.syn_cookies.cookie(peer_addr) else {
            return Err(HandshakeResponseError::MissingCookie);
        };

        if response.validate(&cookie).is_err() {
            return Err(HandshakeResponseError::InvalidSignature);
        }

        Ok(())
    }

    pub(crate) fn prepare_response(
        &self,
        query: &NodeIdHandshakeQuery,
        v2: bool,
    ) -> NodeIdHandshakeResponse {
        if v2 {
            NodeIdHandshakeResponse::new_v2(&query.cookie, &self.node_id, self.genesis_hash)
        } else {
            NodeIdHandshakeResponse::new_v1(&query.cookie, &self.node_id)
        }
    }

    pub(crate) fn prepare_query(&self, peer_addr: &SocketAddrV6) -> Option<NodeIdHandshakeQuery> {
        self.syn_cookies
            .assign(peer_addr)
            .map(|cookie| NodeIdHandshakeQuery { cookie })
    }
}

#[derive(Debug, Clone, Copy)]
enum HandshakeResponseError {
    /// The node tried to connect to itself
    OwnNodeId,
    InvalidGenesis,
    MissingCookie,
    InvalidSignature,
}

impl From<HandshakeResponseError> for DetailType {
    fn from(value: HandshakeResponseError) -> Self {
        match value {
            HandshakeResponseError::OwnNodeId => Self::InvalidNodeId,
            HandshakeResponseError::InvalidGenesis => Self::InvalidGenesis,
            HandshakeResponseError::MissingCookie => Self::MissingCookie,
            HandshakeResponseError::InvalidSignature => Self::InvalidSignature,
        }
    }
}
