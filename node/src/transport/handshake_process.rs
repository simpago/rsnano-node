use super::SynCookies;
use crate::stats::{DetailType, Direction, StatType, Stats};
use rsnano_core::{utils::TEST_ENDPOINT_1, BlockHash, NodeId, PrivateKey};
use rsnano_messages::{
    Message, MessageSerializer, NodeIdHandshake, NodeIdHandshakeQuery, NodeIdHandshakeResponse,
    ProtocolInfo,
};
use rsnano_network::{Channel, TrafficType};
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
    remote_endpoint: SocketAddrV6,
    protocol: ProtocolInfo,
}

impl HandshakeProcess {
    pub(crate) fn new(
        genesis_hash: BlockHash,
        node_id: PrivateKey,
        syn_cookies: Arc<SynCookies>,
        stats: Arc<Stats>,
        remote_endpoint: SocketAddrV6,
        protocol: ProtocolInfo,
    ) -> Self {
        Self {
            genesis_hash,
            node_id,
            syn_cookies,
            stats,
            handshake_received: AtomicBool::new(false),
            remote_endpoint,
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
            remote_endpoint: TEST_ENDPOINT_1,
            protocol: ProtocolInfo::default(),
        }
    }

    pub(crate) async fn initiate_handshake(&self, channel: &Channel) -> Result<(), ()> {
        let endpoint = self.remote_endpoint;
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

    pub(crate) async fn process_handshake(
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
            debug!(
                "Invalid handshake message received ({})",
                self.remote_endpoint
            );
            return HandshakeStatus::Abort;
        }
        if message.query.is_some() && self.handshake_received.load(Ordering::SeqCst) {
            // Second handshake message should be a response only
            self.stats.inc_dir(
                StatType::TcpServer,
                DetailType::HandshakeError,
                Direction::In,
            );
            warn!(
                "Detected multiple handshake queries ({})",
                self.remote_endpoint
            );
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
            log_type, self.remote_endpoint
        );

        if let Some(query) = message.query.clone() {
            // Send response + our own query
            if self
                .send_response(&query, message.is_v2, channel)
                .await
                .is_err()
            {
                // Stop invalid handshake
                return HandshakeStatus::Abort;
            }
            // Fall through and continue handshake
        }
        if let Some(response) = &message.response {
            match self.verify_response(response, &self.remote_endpoint) {
                Ok(()) => {
                    self.stats
                        .inc_dir(StatType::Handshake, DetailType::Ok, Direction::In);
                    return HandshakeStatus::Realtime(response.node_id); // Switch to realtime
                }
                Err(HandshakeResponseError::OwnNodeId) => {
                    warn!(
                        "This node tried to connect to itself. Closing channel ({})",
                        self.remote_endpoint
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
                        self.remote_endpoint, e
                    );
                    return HandshakeStatus::Abort;
                }
            }
        }
        HandshakeStatus::Handshake // Handshake is in progress
    }

    pub(crate) async fn send_response(
        &self,
        query: &NodeIdHandshakeQuery,
        v2: bool,
        channel: &Channel,
    ) -> anyhow::Result<()> {
        let response = self.prepare_response(query, v2);
        let own_query = self.prepare_query(&self.remote_endpoint);

        let handshake_response = Message::NodeIdHandshake(NodeIdHandshake {
            is_v2: own_query.is_some() || response.v2.is_some(),
            query: own_query,
            response: Some(response),
        });

        debug!("Responding to handshake ({})", self.remote_endpoint);

        let mut serializer = MessageSerializer::new(self.protocol);
        let buffer = serializer.serialize(&handshake_response);
        match channel.send_buffer(buffer, TrafficType::Generic).await {
            Ok(_) => {
                self.stats
                    .inc_dir(StatType::TcpServer, DetailType::Handshake, Direction::Out);
                self.stats.inc_dir(
                    StatType::TcpServer,
                    DetailType::HandshakeResponse,
                    Direction::Out,
                );
                Ok(())
            }
            Err(e) => {
                self.stats.inc_dir(
                    StatType::TcpServer,
                    DetailType::HandshakeNetworkError,
                    Direction::In,
                );
                warn!(
                    "Error sending handshake response: {} ({:?})",
                    self.remote_endpoint, e
                );
                Err(e)
            }
        }
    }

    fn verify_response(
        &self,
        response: &NodeIdHandshakeResponse,
        remote_endpoint: &SocketAddrV6,
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

        let Some(cookie) = self.syn_cookies.cookie(remote_endpoint) else {
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

    pub(crate) fn prepare_query(
        &self,
        remote_endpoint: &SocketAddrV6,
    ) -> Option<NodeIdHandshakeQuery> {
        self.syn_cookies
            .assign(remote_endpoint)
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
