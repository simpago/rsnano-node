use super::{
    ChannelEnum, HandshakeProcess, HandshakeStatus, InboundMessageQueue, MessageDeserializer,
    Network, NetworkFilter, SynCookies,
};
use crate::{
    block_processing::BlockProcessor,
    bootstrap::{
        BootstrapInitiator, BulkPullAccountServer, BulkPullServer, BulkPushServer,
        FrontierReqServer,
    },
    config::NodeFlags,
    stats::{DetailType, Direction, StatType, Stats},
    transport::{ChannelMode, NetworkExt, Socket},
    utils::{AsyncRuntime, ThreadPool},
    NetworkParams,
};
use async_trait::async_trait;
use rsnano_core::{
    utils::{OutputListenerMt, OutputTrackerMt, NULL_ENDPOINT},
    Account, KeyPair,
};
use rsnano_ledger::Ledger;
use rsnano_messages::*;
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
    allow_bootstrap: bool,
    notify_stop: Notify,
    last_keepalive: Mutex<Option<Keepalive>>,
    network: Weak<Network>,
    inbound_queue: Arc<InboundMessageQueue>,
    handshake_process: HandshakeProcess,
    initiate_handshake_listener: OutputListenerMt<()>,
    publish_filter: Arc<NetworkFilter>,
    runtime: Arc<AsyncRuntime>,
    ledger: Arc<Ledger>,
    workers: Arc<dyn ThreadPool>,
    block_processor: Arc<BlockProcessor>,
    bootstrap_initiator: Arc<BootstrapInitiator>,
    flags: NodeFlags,
}

static NEXT_UNIQUE_ID: AtomicUsize = AtomicUsize::new(0);

impl ResponseServer {
    pub fn new(
        network: &Arc<Network>,
        inbound_queue: Arc<InboundMessageQueue>,
        socket: Arc<Socket>,
        publish_filter: Arc<NetworkFilter>,
        network_params: Arc<NetworkParams>,
        stats: Arc<Stats>,
        allow_bootstrap: bool,
        syn_cookies: Arc<SynCookies>,
        node_id: KeyPair,
        runtime: Arc<AsyncRuntime>,
        ledger: Arc<Ledger>,
        workers: Arc<dyn ThreadPool>,
        block_processor: Arc<BlockProcessor>,
        bootstrap_initiator: Arc<BootstrapInitiator>,
        flags: NodeFlags,
    ) -> Self {
        let network_constants = network_params.network.clone();
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
            stats: stats.clone(),
            disable_bootstrap_bulk_pull_server: false,
            allow_bootstrap,
            notify_stop: Notify::new(),
            last_keepalive: Mutex::new(None),
            initiate_handshake_listener: OutputListenerMt::new(),
            publish_filter,
            runtime,
            ledger,
            workers,
            block_processor,
            bootstrap_initiator,
            flags,
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

impl Drop for ResponseServer {
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
    fn process_realtime(&self, message: DeserializedMessage) -> ProcessResult;
    fn process_bootstrap(&self, message: DeserializedMessage) -> ProcessResult;
}

pub enum ProcessResult {
    Abort,
    Progress,
    Pause,
}

#[async_trait]
impl ResponseServerExt for Arc<ResponseServer> {
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

        let mut message_deserializer = MessageDeserializer::new(
            self.network_params.network.protocol_info(),
            self.network_params.network.work.clone(),
            self.publish_filter.clone(),
            self.socket.clone(),
        );

        loop {
            if self.is_stopped() {
                break;
            }

            let result = tokio::select! {
                i = message_deserializer.read() => i,
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
            return self.process_realtime(message);
        }

        // The server will switch to bootstrap mode immediately after processing the first bootstrap message, thus no `else if`
        if self.is_bootstrap_connection() {
            return self.process_bootstrap(message);
        }
        debug_assert!(false);
        ProcessResult::Abort
    }

    fn process_realtime(&self, message: DeserializedMessage) -> ProcessResult {
        let process = match &message.message {
            Message::Keepalive(keepalive) => {
                self.set_last_keepalive(Some(keepalive.clone()));
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

    fn process_bootstrap(&self, message: DeserializedMessage) -> ProcessResult {
        match &message.message {
            Message::BulkPull(payload) => {
                if self.flags.disable_bootstrap_bulk_pull_server {
                    return ProcessResult::Progress;
                }

                let payload = payload.clone();
                let connection = Arc::clone(self);
                let ledger = Arc::clone(&self.ledger);
                let thread_pool2 = self.workers.clone();
                let runtime = self.runtime.clone();
                self.workers.push_task(Box::new(move || {
                    // TODO from original code: Add completion callback to bulk pull server
                    // TODO from original code: There should be no need to re-copy message as unique pointer, refactor those bulk/frontier pull/push servers
                    let mut bulk_pull_server =
                        BulkPullServer::new(payload, connection, ledger, thread_pool2, runtime);
                    bulk_pull_server.send_next();
                }));

                ProcessResult::Pause
            }
            Message::BulkPullAccount(payload) => {
                if self.flags.disable_bootstrap_bulk_pull_server {
                    return ProcessResult::Progress;
                }
                let payload = payload.clone();
                let connection = Arc::clone(self);
                let ledger = self.ledger.clone();
                let thread_pool2 = self.workers.clone();
                let runtime = self.runtime.clone();
                self.workers.push_task(Box::new(move || {
                    // original code TODO: Add completion callback to bulk pull server
                    // original code TODO: There should be no need to re-copy message as unique pointer, refactor those bulk/frontier pull/push servers
                    let bulk_pull_account_server = BulkPullAccountServer::new(
                        connection,
                        payload,
                        thread_pool2,
                        ledger,
                        runtime,
                    );
                    bulk_pull_account_server.send_frontier();
                }));

                ProcessResult::Pause
            }
            Message::BulkPush => {
                // original code TODO: Add completion callback to bulk pull server
                let bulk_push_server = BulkPushServer::new(
                    self.runtime.clone(),
                    Arc::clone(self),
                    self.workers.clone(),
                    self.block_processor.clone(),
                    self.bootstrap_initiator.clone(),
                    self.stats.clone(),
                    self.network_params.network.work.clone(),
                );

                self.workers.push_task(Box::new(move || {
                    bulk_push_server.throttled_receive();
                }));

                ProcessResult::Pause
            }
            Message::FrontierReq(payload) => {
                // original code TODO: There should be no need to re-copy message as unique pointer, refactor those bulk/frontier pull/push servers
                let response = FrontierReqServer::new(
                    Arc::clone(self),
                    payload.clone(),
                    self.workers.clone(),
                    self.ledger.clone(),
                    self.runtime.clone(),
                );
                self.workers.push_task(Box::new(move || {
                    response.send_next();
                }));

                ProcessResult::Pause
            }
            _ => ProcessResult::Progress,
        }
    }
}
