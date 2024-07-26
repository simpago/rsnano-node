use super::{
    ChannelEnum, HandshakeProcess, HandshakeStatus, InboundMessageQueue, MessageDeserializer,
    Network, NetworkFilter, SynCookies,
};
use crate::{
    bootstrap::BootstrapMessageVisitorFactory,
    stats::{DetailType, Direction, StatType, Stats},
    transport::{ChannelMode, NetworkExt, Socket, SocketExtensions},
    NetworkParams,
};
use async_trait::async_trait;
use rsnano_core::{
    utils::{OutputListenerMt, OutputTrackerMt, NULL_ENDPOINT, TEST_ENDPOINT_1},
    Account, KeyPair, Networks,
};
use rsnano_messages::*;
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddrV6,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex, Weak,
    },
    time::{Duration, Instant, SystemTime},
};
use tokio::sync::Notify;
use tracing::{debug, info};

#[derive(Clone, Debug, Deserialize, Serialize)]
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

pub trait ResponseServer {}

pub struct ResponseServerImpl {
    channel: Mutex<Option<Arc<ChannelEnum>>>,
    pub socket: Arc<Socket>,
    stopped: AtomicBool,
    pub disable_bootstrap_listener: bool,
    pub connections_max: usize,

    // Remote enpoint used to remove response channel even after socket closing
    remote_endpoint: Mutex<SocketAddrV6>,

    network_params: Arc<NetworkParams>,
    last_telemetry_req: Mutex<Option<Instant>>,
    unique_id: usize,
    stats: Arc<Stats>,
    pub disable_bootstrap_bulk_pull_server: bool,
    message_visitor_factory: Arc<BootstrapMessageVisitorFactory>,
    message_deserializer: Arc<MessageDeserializer<Arc<Socket>>>,
    allow_bootstrap: bool,
    notify_stop: Notify,
    last_keepalive: Mutex<Option<Keepalive>>,
    network: Weak<Network>,
    inbound_queue: Arc<InboundMessageQueue>,
    handshake_process: HandshakeProcess,
    initiate_handshake_listener: OutputListenerMt<()>,
}

static NEXT_UNIQUE_ID: AtomicUsize = AtomicUsize::new(0);

impl ResponseServerImpl {
    pub fn new(
        network: &Arc<Network>,
        inbound_queue: Arc<InboundMessageQueue>,
        socket: Arc<Socket>,
        publish_filter: Arc<NetworkFilter>,
        network_params: Arc<NetworkParams>,
        stats: Arc<Stats>,
        message_visitor_factory: Arc<BootstrapMessageVisitorFactory>,
        allow_bootstrap: bool,
        syn_cookies: Arc<SynCookies>,
        node_id: KeyPair,
    ) -> Self {
        let network_constants = network_params.network.clone();
        let socket_clone = Arc::clone(&socket);
        debug!(
            socket_id = socket.socket_id,
            "Cloning socket in TcpServer constructor"
        );
        let remote_endpoint = socket.get_remote().unwrap_or(NULL_ENDPOINT);
        Self {
            network: Arc::downgrade(network),
            inbound_queue,
            socket,
            channel: Mutex::new(None),
            stopped: AtomicBool::new(false),
            disable_bootstrap_listener: false,
            connections_max: 64,
            remote_endpoint: Mutex::new(remote_endpoint),
            last_telemetry_req: Mutex::new(None),
            handshake_process: HandshakeProcess::new(
                network_params.ledger.genesis.hash(),
                node_id.clone(),
                syn_cookies,
                stats.clone(),
                remote_endpoint,
                network_constants.protocol_info(),
            ),
            network_params,
            unique_id: NEXT_UNIQUE_ID.fetch_add(1, Ordering::Relaxed),
            stats,
            disable_bootstrap_bulk_pull_server: false,
            message_visitor_factory,
            message_deserializer: Arc::new(MessageDeserializer::new(
                network_constants.protocol_info(),
                network_constants.work.clone(),
                publish_filter,
                socket_clone,
            )),
            allow_bootstrap,
            notify_stop: Notify::new(),
            last_keepalive: Mutex::new(None),
            initiate_handshake_listener: OutputListenerMt::new(),
        }
    }

    pub fn new_null() -> Self {
        Self {
            channel: Mutex::new(None),
            socket: Socket::new_null(),
            stopped: AtomicBool::new(false),
            disable_bootstrap_listener: true,
            connections_max: 1,
            remote_endpoint: Mutex::new(TEST_ENDPOINT_1),
            network_params: Arc::new(NetworkParams::new(Networks::NanoDevNetwork)),
            last_telemetry_req: Mutex::new(None),
            unique_id: 42,
            stats: Arc::new(Stats::default()),
            disable_bootstrap_bulk_pull_server: true,
            message_visitor_factory: Arc::new(BootstrapMessageVisitorFactory::new_null()),
            message_deserializer: Arc::new(MessageDeserializer::new_null()),
            allow_bootstrap: false,
            notify_stop: Notify::new(),
            last_keepalive: Mutex::new(None),
            network: Arc::downgrade(&Arc::new(Network::new_null())),
            inbound_queue: Arc::new(InboundMessageQueue::default()),
            handshake_process: HandshakeProcess::new_null(),
            initiate_handshake_listener: OutputListenerMt::new(),
        }
    }

    pub fn track_handshake_initiation(&self) -> Arc<OutputTrackerMt<()>> {
        self.initiate_handshake_listener.track()
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }

    pub fn stop(&self) {
        if !self.stopped.swap(true, Ordering::SeqCst) {
            self.socket.close();
            self.notify_stop.notify_one();
        }
    }

    pub fn remote_endpoint(&self) -> SocketAddrV6 {
        *self.remote_endpoint.lock().unwrap()
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

    fn to_bootstrap_connection(&self) -> bool {
        if !self.allow_bootstrap {
            return false;
        }

        if self.socket.mode() != ChannelMode::Undefined {
            return false;
        }

        if self.disable_bootstrap_listener {
            return false;
        }

        let Some(network) = self.network.upgrade() else {
            return false;
        };

        if network.count_by_mode(ChannelMode::Bootstrap) >= self.connections_max {
            return false;
        }

        self.socket.set_mode(ChannelMode::Bootstrap);
        debug!("Switched to bootstrap mode ({})", self.remote_endpoint());
        true
    }

    fn set_last_telemetry_req(&self) {
        let mut lk = self.last_telemetry_req.lock().unwrap();
        *lk = Some(Instant::now());
    }

    pub fn unique_id(&self) -> usize {
        self.unique_id
    }

    fn is_undefined_connection(&self) -> bool {
        self.socket.mode() == ChannelMode::Undefined
    }

    fn is_bootstrap_connection(&self) -> bool {
        self.socket.is_bootstrap_connection()
    }

    fn is_realtime_connection(&self) -> bool {
        self.socket.is_realtime_connection()
    }

    fn queue_realtime(&self, message: DeserializedMessage) {
        let channel = self.channel.lock().unwrap().as_ref().unwrap().clone();
        channel.set_last_packet_received(SystemTime::now());
        self.inbound_queue.put(message, channel);
        // TODO: Throttle if not added
    }

    fn set_last_keepalive(&self, keepalive: Option<Keepalive>) {
        *self.last_keepalive.lock().unwrap() = keepalive;
    }

    pub fn get_last_keepalive(&self) -> Option<Keepalive> {
        self.last_keepalive.lock().unwrap().clone()
    }

    pub fn pop_last_keepalive(&self) -> Option<Keepalive> {
        self.last_keepalive.lock().unwrap().take()
    }

    pub fn set_channel(&self, channel: Arc<ChannelEnum>) {
        *self.channel.lock().unwrap() = Some(channel);
    }

    pub async fn initiate_handshake(&self) {
        self.initiate_handshake_listener.emit(());
        if self
            .handshake_process
            .initiate_handshake(&self.socket)
            .await
            .is_err()
        {
            self.stop();
        }
    }
}

impl Drop for ResponseServerImpl {
    fn drop(&mut self) {
        let remote_ep = { *self.remote_endpoint.lock().unwrap() };
        debug!("Exiting server: {}", remote_ep);
        self.stop();
    }
}

pub trait RealtimeMessageVisitor: MessageVisitor {
    fn process(&self) -> bool;
    fn as_message_visitor(&mut self) -> &mut dyn MessageVisitor;
}

pub trait BootstrapMessageVisitor: MessageVisitor {
    fn processed(&self) -> bool;
    fn as_message_visitor(&mut self) -> &mut dyn MessageVisitor;
}

#[async_trait]
pub trait ResponseServerExt {
    fn timeout(&self);

    fn to_realtime_connection(&self, node_id: &Account) -> bool;
    async fn run(&self);
    async fn process_message(&self, message: DeserializedMessage) -> ProcessResult;
}

pub enum ProcessResult {
    Abort,
    Progress,
    Pause,
}

#[async_trait]
impl ResponseServerExt for Arc<ResponseServerImpl> {
    fn to_realtime_connection(&self, node_id: &Account) -> bool {
        if self.socket.mode() != ChannelMode::Undefined {
            return false;
        }

        let Some(network) = self.network.upgrade() else {
            return false;
        };

        let Some(remote) = self.socket.get_remote() else {
            return false;
        };

        network.upgrade_to_realtime_connection(&remote, *node_id);
        debug!("Switched to realtime mode ({})", self.remote_endpoint());
        return true;
    }

    fn timeout(&self) {
        if self.socket.has_timed_out() {
            debug!("Closing TCP server due to timeout");
            self.socket.close();
        }
    }

    async fn run(&self) {
        // Set remote_endpoint
        {
            let mut guard = self.remote_endpoint.lock().unwrap();
            if guard.port() == 0 {
                if let Some(ep) = self.socket.get_remote() {
                    *guard = ep;
                }
                //debug_assert!(guard.port() != 0);
            }
            debug!("Starting server: {}", guard.port());
        }

        loop {
            if self.is_stopped() {
                break;
            }

            let result = tokio::select! {
                i = self.message_deserializer.read() => i,
                _ = self.notify_stop.notified() => Err(ParseMessageError::Stopped)
            };

            let result = match result {
                Ok(msg) => self.process_message(msg).await,
                Err(ParseMessageError::DuplicatePublishMessage) => {
                    // Avoid too much noise about `duplicate_publish_message` errors
                    self.stats.inc_dir(
                        StatType::Filter,
                        DetailType::DuplicatePublishMessage,
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
                    info!(
                        "Error reading message: {:?} ({})",
                        e,
                        self.remote_endpoint()
                    );
                    ProcessResult::Abort
                }
            };

            match result {
                ProcessResult::Abort => {
                    self.stop();
                    break;
                }
                ProcessResult::Progress => {}
                ProcessResult::Pause => {
                    break;
                }
            }
        }
    }

    async fn process_message(&self, message: DeserializedMessage) -> ProcessResult {
        self.stats.inc_dir(
            StatType::TcpServer,
            DetailType::from(message.message.message_type()),
            Direction::In,
        );

        debug_assert!(
            self.is_undefined_connection()
                || self.is_realtime_connection()
                || self.is_bootstrap_connection()
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
        if self.is_undefined_connection() {
            let result = match &message.message {
                Message::BulkPull(_)
                | Message::BulkPullAccount(_)
                | Message::BulkPush
                | Message::FrontierReq(_) => HandshakeStatus::Bootstrap,
                Message::NodeIdHandshake(payload) => {
                    self.handshake_process
                        .process_handshake(payload, &self.socket)
                        .await
                }

                _ => HandshakeStatus::Abort,
            };

            match result {
                HandshakeStatus::Abort => {
                    self.stats.inc_dir(
                        StatType::TcpServer,
                        DetailType::HandshakeAbort,
                        Direction::In,
                    );
                    debug!(
                        "Aborting handshake: {:?} ({})",
                        message.message.message_type(),
                        self.remote_endpoint()
                    );
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
                            self.remote_endpoint()
                        );
                        return ProcessResult::Abort;
                    }
                    self.queue_realtime(message);
                    return ProcessResult::Progress; // Continue receiving new messages
                }
                HandshakeStatus::Bootstrap => {
                    if !self.to_bootstrap_connection() {
                        self.stats.inc_dir(
                            StatType::TcpServer,
                            DetailType::HandshakeError,
                            Direction::In,
                        );
                        debug!(
                            "Error switching to bootstrap mode: {:?} ({})",
                            message.message.message_type(),
                            self.remote_endpoint()
                        );
                        return ProcessResult::Abort;
                    } else {
                        // Fall through to process the bootstrap message
                    }
                }
            }
        } else if self.is_realtime_connection() {
            let mut realtime_visitor = self
                .message_visitor_factory
                .realtime_visitor(Arc::clone(self));
            realtime_visitor.received(&message.message);
            if realtime_visitor.process() {
                self.queue_realtime(message);
            }
            return ProcessResult::Progress;
        }

        // The server will switch to bootstrap mode immediately after processing the first bootstrap message, thus no `else if`
        if self.is_bootstrap_connection() {
            let mut bootstrap_visitor = self
                .message_visitor_factory
                .bootstrap_visitor(Arc::clone(self));
            let processed = bootstrap_visitor.received(&message.message);

            // Pause receiving new messages if bootstrap serving started
            return if processed {
                ProcessResult::Pause
            } else {
                ProcessResult::Progress
            };
        }
        debug_assert!(false);
        ProcessResult::Abort
    }
}

pub struct RealtimeMessageVisitorImpl {
    server: Arc<ResponseServerImpl>,
    stats: Arc<Stats>,
    process: bool,
}

impl RealtimeMessageVisitorImpl {
    pub fn new(server: Arc<ResponseServerImpl>, stats: Arc<Stats>) -> Self {
        Self {
            server,
            stats,
            process: false,
        }
    }
}

impl MessageVisitor for RealtimeMessageVisitorImpl {
    fn received(&mut self, message: &Message) {
        match message {
            Message::Keepalive(keepalive) => {
                self.process = true;
                self.server.set_last_keepalive(Some(keepalive.clone()));
            }
            Message::Publish(_)
            | Message::AscPullAck(_)
            | Message::AscPullReq(_)
            | Message::ConfirmAck(_)
            | Message::ConfirmReq(_)
            | Message::FrontierReq(_)
            | Message::TelemetryAck(_) => self.process = true,
            Message::TelemetryReq => {
                // Only handle telemetry requests if they are outside of the cooldown period
                if self.server.is_outside_cooldown_period() {
                    self.server.set_last_telemetry_req();
                    self.process = true;
                } else {
                    self.stats.inc_dir(
                        StatType::Telemetry,
                        DetailType::RequestWithinProtectionCacheZone,
                        Direction::In,
                    );
                }
            }
            _ => {}
        }
    }
}

impl RealtimeMessageVisitor for RealtimeMessageVisitorImpl {
    fn process(&self) -> bool {
        self.process
    }

    fn as_message_visitor(&mut self) -> &mut dyn MessageVisitor {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "todo"]
    async fn can_track_handshake_initiation() {
        let response_server = ResponseServerImpl::new_null();
        let handshake_tracker = response_server.track_handshake_initiation();

        response_server.initiate_handshake().await;

        assert_eq!(handshake_tracker.output().len(), 1);
    }
}
