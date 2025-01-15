use super::{HandshakeProcess, HandshakeStatus, InboundMessageQueue, LatestKeepalives, SynCookies};
use crate::{
    stats::{DetailType, Direction, StatType, Stats},
    NetworkParams,
};
use rsnano_core::{NodeId, PrivateKey};
use rsnano_messages::*;
use rsnano_network::{Channel, ChannelDirection, ChannelMode, Network, TcpChannelAdapter};
use std::{
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant},
};
use tracing::debug;

#[derive(Clone, Debug, PartialEq)]
pub struct TcpConfig {
    pub max_inbound_connections: usize,
    pub max_outbound_connections: usize,
    pub max_attempts: usize,
    pub max_attempts_per_ip: usize,
    pub connect_timeout: Duration,
}

impl TcpConfig {
    pub fn for_dev_network() -> Self {
        Self {
            max_inbound_connections: 128,
            max_outbound_connections: 128,
            max_attempts: 128,
            max_attempts_per_ip: 128,
            connect_timeout: Duration::from_secs(5),
        }
    }
}

impl Default for TcpConfig {
    fn default() -> Self {
        Self {
            max_inbound_connections: 2048,
            max_outbound_connections: 2048,
            max_attempts: 60,
            max_attempts_per_ip: 1,
            connect_timeout: Duration::from_secs(60),
        }
    }
}

pub struct ResponseServer {
    channel_adapter: Arc<TcpChannelAdapter>,
    channel: Arc<Channel>,
    network_params: Arc<NetworkParams>,
    last_telemetry_req: Mutex<Option<Instant>>,
    stats: Arc<Stats>,
    network: Arc<RwLock<Network>>,
    inbound_queue: Arc<InboundMessageQueue>,
    network_filter: Arc<NetworkFilter>,
    latest_keepalives: Arc<Mutex<LatestKeepalives>>,
    syn_cookies: Arc<SynCookies>,
    node_id: PrivateKey,
}

impl ResponseServer {
    pub fn new(
        network: Arc<RwLock<Network>>,
        inbound_queue: Arc<InboundMessageQueue>,
        channel_adapter: Arc<TcpChannelAdapter>,
        network_filter: Arc<NetworkFilter>,
        network_params: Arc<NetworkParams>,
        stats: Arc<Stats>,
        syn_cookies: Arc<SynCookies>,
        node_id: PrivateKey,
        latest_keepalives: Arc<Mutex<LatestKeepalives>>,
    ) -> Self {
        let channel = channel_adapter.channel.clone();
        Self {
            network,
            inbound_queue,
            channel,
            channel_adapter,
            last_telemetry_req: Mutex::new(None),
            syn_cookies: syn_cookies.clone(),
            node_id: node_id.clone(),
            network_params,
            stats: stats.clone(),
            network_filter,
            latest_keepalives,
        }
    }

    fn is_outside_cooldown_period(&self) -> bool {
        let lock = self.last_telemetry_req.lock().unwrap();
        match *lock {
            Some(last_req) => {
                last_req.elapsed() >= self.network_params.network.telemetry_request_cooldown
            }
            None => true,
        }
    }

    fn set_last_telemetry_req(&self) {
        let mut lk = self.last_telemetry_req.lock().unwrap();
        *lk = Some(Instant::now());
    }

    fn set_last_keepalive(&self, keepalive: Keepalive) {
        self.latest_keepalives
            .lock()
            .unwrap()
            .insert(self.channel.channel_id(), keepalive);
    }

    pub async fn run(&self) {
        let handshake_process = HandshakeProcess::new(
            self.network_params.ledger.genesis_block.hash(),
            self.node_id.clone(),
            self.syn_cookies.clone(),
            self.stats.clone(),
            self.network_params.network.protocol_info(),
        );

        let mut receiver = NanoDataReceiver::new(self.channel.clone(), handshake_process);
        receiver.initialize();

        debug!(peer = %self.channel.peer_addr(), "Starting response server");
        let mut buffer = [0u8; 1024];

        let mut message_deserializer = MessageDeserializer::new(
            self.network_params.network.protocol_info(),
            self.network_filter.clone(),
            self.network_params.network.work.clone(),
        );

        let mut first_message = true;

        loop {
            if !self.channel.is_alive() {
                return;
            }

            if let Err(e) = self.channel_adapter.readable().await {
                debug!(
                    "Error reading buffer: {:?} ({})",
                    e,
                    self.channel.peer_addr()
                );
                self.channel.close();
                return;
            }

            let read_count = match self.channel_adapter.try_read(&mut buffer) {
                Ok(n) => n,
                Err(e) => {
                    debug!(
                        "Error reading buffer: {:?} ({})",
                        e,
                        self.channel.peer_addr()
                    );
                    self.channel.close();
                    return;
                }
            };

            // TODO: push read bytes into channel and then call a callback inside channel:
            //self.channel.receive_data(&buffer[..read_count])

            message_deserializer.push(&buffer[..read_count]);
            while let Some(result) = message_deserializer.try_deserialize() {
                let result = match result {
                    Ok(msg) => {
                        if first_message {
                            // TODO: if version using changes => peer misbehaved!
                            self.channel
                                .set_protocol_version(msg.protocol.version_using);
                            first_message = false;
                        }
                        self.process_message(msg.message, &receiver)
                    }
                    Err(ParseMessageError::DuplicatePublishMessage) => {
                        // Avoid too much noise about `duplicate_publish_message` errors
                        self.stats.inc_dir(
                            StatType::Filter,
                            DetailType::DuplicatePublishMessage,
                            Direction::In,
                        );
                        ProcessResult::Progress
                    }
                    Err(ParseMessageError::DuplicateConfirmAckMessage) => {
                        self.stats.inc_dir(
                            StatType::Filter,
                            DetailType::DuplicateConfirmAckMessage,
                            Direction::In,
                        );
                        ProcessResult::Progress
                    }
                    Err(ParseMessageError::InsufficientWork) => {
                        // IO error or critical error when deserializing message
                        self.stats.inc_dir(
                            StatType::Error,
                            DetailType::InsufficientWork,
                            Direction::In,
                        );
                        ProcessResult::Progress
                    }
                    Err(e) => {
                        // IO error or critical error when deserializing message
                        self.stats
                            .inc_dir(StatType::Error, DetailType::from(&e), Direction::In);
                        debug!(
                            "Error reading message: {:?} ({})",
                            e,
                            self.channel.peer_addr()
                        );
                        ProcessResult::Abort
                    }
                };

                match result {
                    ProcessResult::Abort => {
                        self.channel.close();
                        break;
                    }
                    ProcessResult::Progress => {}
                }
            }
        }
    }

    fn process_message(&self, message: Message, receiver: &NanoDataReceiver) -> ProcessResult {
        self.stats.inc_dir(
            StatType::TcpServer,
            DetailType::from(message.message_type()),
            Direction::In,
        );

        /*
         * Server initially starts in undefined state, where it waits for either a handshake or booststrap request message
         * If the server receives a handshake (and it is successfully validated) it will switch to a realtime mode.
         * In realtime mode messages are deserialized and queued to `tcp_message_manager` for further processing.
         * In realtime mode any bootstrap requests are ignored.
         *
         * If the server receives a bootstrap request before receiving a handshake, it will switch to a bootstrap mode.
         * In bootstrap mode once a valid bootstrap request message is received, the server will start a corresponding bootstrap server and pass control to that server.
         * Once that server finishes its task, control is passed back to this server to read and process any subsequent messages.
         * In bootstrap mode any realtime messages are ignored
         */
        if self.channel.mode() == ChannelMode::Undefined {
            let result = match &message {
                Message::BulkPull(_)
                | Message::BulkPullAccount(_)
                | Message::BulkPush
                | Message::FrontierReq(_) => HandshakeStatus::Bootstrap,
                Message::NodeIdHandshake(payload) => receiver
                    .handshake_process
                    .process_handshake(payload, &self.channel),

                _ => HandshakeStatus::Abort,
            };

            match result {
                HandshakeStatus::Abort | HandshakeStatus::AbortOwnNodeId => {
                    self.stats.inc_dir(
                        StatType::TcpServer,
                        DetailType::HandshakeAbort,
                        Direction::In,
                    );
                    debug!(
                        "Aborting handshake: {:?} ({})",
                        message.message_type(),
                        self.channel.peer_addr()
                    );
                    if matches!(result, HandshakeStatus::AbortOwnNodeId) {
                        if let Some(peering_addr) = self.channel.peering_addr() {
                            self.network.write().unwrap().perma_ban(peering_addr);
                        }
                    }
                    return ProcessResult::Abort;
                }
                HandshakeStatus::Handshake => {
                    return ProcessResult::Progress; // Continue handshake
                }
                HandshakeStatus::Realtime(node_id) => {
                    if !self.to_realtime_connection(&node_id) {
                        self.stats.inc_dir(
                            StatType::TcpServer,
                            DetailType::HandshakeError,
                            Direction::In,
                        );
                        debug!(
                            "Error switching to realtime mode ({})",
                            self.channel.peer_addr()
                        );
                        return ProcessResult::Abort;
                    }
                    self.queue_realtime(message);
                    return ProcessResult::Progress; // Continue receiving new messages
                }
                HandshakeStatus::Bootstrap => {
                    debug!(peer = ?self.channel.peer_addr(), "Legacy bootstrap isn't supported. Closing connection");
                    // Legacy bootstrap is not supported anymore
                    return ProcessResult::Abort;
                }
            }
        } else if self.channel.mode() == ChannelMode::Realtime {
            return self.process_realtime(message);
        }

        debug_assert!(false);
        ProcessResult::Abort
    }

    fn to_realtime_connection(&self, node_id: &NodeId) -> bool {
        if self.channel.mode() != ChannelMode::Undefined {
            return false;
        }

        let result = self
            .network
            .read()
            .unwrap()
            .upgrade_to_realtime_connection(self.channel.channel_id(), *node_id);

        if let Some((channel, observers)) = result {
            for observer in observers {
                observer(channel.clone());
            }

            self.stats
                .inc(StatType::TcpChannels, DetailType::ChannelAccepted);

            debug!(
                "Switched to realtime mode (addr: {}, node_id: {})",
                self.channel.peer_addr(),
                node_id
            );
            true
        } else {
            debug!(
                channel_id = ?self.channel.channel_id(),
                peer = %self.channel.peer_addr(),
                %node_id,
                "Could not upgrade channel to realtime connection, because another channel for the same node ID was found",
            );
            false
        }
    }

    fn process_realtime(&self, message: Message) -> ProcessResult {
        let process = match &message {
            Message::Keepalive(keepalive) => {
                self.set_last_keepalive(keepalive.clone());
                true
            }
            Message::Publish(_)
            | Message::AscPullAck(_)
            | Message::AscPullReq(_)
            | Message::ConfirmAck(_)
            | Message::ConfirmReq(_)
            | Message::FrontierReq(_)
            | Message::TelemetryAck(_) => true,
            Message::TelemetryReq => {
                // Only handle telemetry requests if they are outside of the cooldown period
                if self.is_outside_cooldown_period() {
                    self.set_last_telemetry_req();
                    true
                } else {
                    self.stats.inc_dir(
                        StatType::Telemetry,
                        DetailType::RequestWithinProtectionCacheZone,
                        Direction::In,
                    );
                    false
                }
            }
            _ => false,
        };

        if process {
            self.queue_realtime(message);
        }

        ProcessResult::Progress
    }

    fn queue_realtime(&self, message: Message) {
        self.inbound_queue.put(message, self.channel.clone());
        // TODO: Throttle if not added
    }
}

impl Drop for ResponseServer {
    fn drop(&mut self) {
        debug!("Exiting server: {}", self.channel.peer_addr());
        self.channel.close();
    }
}

pub enum ProcessResult {
    Abort,
    Progress,
}

/// Receives raw data (from a TCP stream) and processes it
struct NanoDataReceiver {
    channel: Arc<Channel>,
    handshake_process: HandshakeProcess,
}

impl NanoDataReceiver {
    fn new(channel: Arc<Channel>, handshake_process: HandshakeProcess) -> Self {
        Self {
            channel,
            handshake_process,
        }
    }

    pub fn initialize(&mut self) {
        if self.channel.direction() == ChannelDirection::Outbound {
            self.initiate_handshake();
        }
    }

    fn initiate_handshake(&self) {
        if self
            .handshake_process
            .initiate_handshake(&self.channel)
            .is_err()
        {
            self.channel.close();
        }
    }
}
