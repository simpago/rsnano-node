use rsnano_core::{work::WorkPoolImpl, Amount, Networks, WalletId};
use rsnano_node::{
    config::{NodeConfig, NodeFlags},
    node::{Node, NodeExt},
    transport::{NullSocketObserver, PeerConnectorExt},
    unique_path,
    utils::AsyncRuntime,
    wallets::WalletsExt,
    NetworkParams,
};
use std::{
    net::TcpListener,
    sync::{
        atomic::{AtomicU16, Ordering},
        Arc, OnceLock,
    },
    thread::sleep,
    time::{Duration, Instant},
};
use tracing_subscriber::EnvFilter;

pub(crate) struct System {
    runtime: Arc<AsyncRuntime>,
    network_params: NetworkParams,
    pub work: Arc<WorkPoolImpl>,
    nodes: Vec<Arc<Node>>,
}

impl System {
    pub(crate) fn new() -> Self {
        init_tracing();
        let network_params = NetworkParams::new(Networks::NanoDevNetwork);

        Self {
            runtime: Arc::new(AsyncRuntime::default()),
            work: Arc::new(WorkPoolImpl::new(
                network_params.work.clone(),
                1,
                Duration::ZERO,
            )),
            network_params,
            nodes: Vec::new(),
        }
    }

    pub(crate) fn default_config() -> NodeConfig {
        let network_params = NetworkParams::new(Networks::NanoDevNetwork);
        let port = get_available_port();
        let mut config = NodeConfig::new(Some(port), &network_params, 1);
        config.representative_vote_weight_minimum = Amount::zero();
        config
    }

    pub(crate) fn build_node<'a>(&'a mut self) -> NodeBuilder<'a> {
        NodeBuilder {
            system: self,
            config: None,
            flags: None,
            disconnected: false,
        }
    }

    pub(crate) fn make_disconnected_node(&mut self) -> Arc<Node> {
        self.build_node().disconnected().finish()
    }

    pub(crate) fn make_node(&mut self) -> Arc<Node> {
        self.build_node().finish()
    }

    fn make_node_with(
        &mut self,
        config: NodeConfig,
        flags: NodeFlags,
        disconnected: bool,
    ) -> Arc<Node> {
        let node = self.new_node(config, flags);
        let wallet_id = WalletId::random();
        node.wallets.create(wallet_id);
        node.start();
        self.nodes.push(node.clone());

        if self.nodes.len() > 1 && !disconnected {
            self.nodes[0]
                .peer_connector
                .connect_to(node.tcp_listener.local_address());
        }
        node
    }

    fn new_node(&self, config: NodeConfig, flags: NodeFlags) -> Arc<Node> {
        let path = unique_path().expect("Could not get a unique path");

        Arc::new(Node::new(
            self.runtime.clone(),
            path,
            config,
            self.network_params.clone(),
            flags,
            self.work.clone(),
            Arc::new(NullSocketObserver::new()),
            Box::new(|_, _, _, _, _, _| {}),
            Box::new(|_, _| {}),
            Box::new(|_, _, _, _| {}),
        ))
    }

    fn stop(&mut self) {
        for node in &self.nodes {
            node.stop();
            std::fs::remove_dir_all(&node.application_path)
                .expect("Could not delete node data dir");
        }
        self.work.stop();
    }
}

impl Drop for System {
    fn drop(&mut self) {
        self.stop();
    }
}

pub(crate) struct NodeBuilder<'a> {
    system: &'a mut System,
    config: Option<NodeConfig>,
    flags: Option<NodeFlags>,
    disconnected: bool,
}

impl<'a> NodeBuilder<'a> {
    pub(crate) fn config(mut self, cfg: NodeConfig) -> Self {
        self.config = Some(cfg);
        self
    }

    pub(crate) fn flags(mut self, flags: NodeFlags) -> Self {
        self.flags = Some(flags);
        self
    }

    pub(crate) fn disconnected(mut self) -> Self {
        self.disconnected = true;
        self
    }

    pub(crate) fn finish(self) -> Arc<Node> {
        let config = self.config.unwrap_or_else(|| System::default_config());
        let flags = self.flags.unwrap_or_default();
        self.system.make_node_with(config, flags, self.disconnected)
    }
}

static START_PORT: AtomicU16 = AtomicU16::new(1025);

pub(crate) fn get_available_port() -> u16 {
    let start = START_PORT.fetch_add(1, Ordering::SeqCst);
    (start..65535)
        .find(|port| is_port_available(*port))
        .expect("Could not find an available port")
}

fn is_port_available(port: u16) -> bool {
    match TcpListener::bind(("127.0.0.1", port)) {
        Ok(_) => true,
        Err(_) => false,
    }
}

pub(crate) fn assert_never(duration: Duration, mut check: impl FnMut() -> bool) {
    let start = Instant::now();
    while start.elapsed() < duration {
        if check() {
            panic!("never check failed");
        }
        sleep(Duration::from_millis(50));
    }
}

pub(crate) fn assert_timely<F>(timeout: Duration, mut check: F, error_message: &str)
where
    F: FnMut() -> bool,
{
    let start = Instant::now();
    while start.elapsed() < timeout {
        if check() {
            return;
        }
        sleep(Duration::from_millis(50));
    }
    panic!("{}", error_message);
}

pub(crate) fn assert_timely_eq<T, F>(timeout: Duration, mut check: F, expected: T)
where
    T: PartialEq,
    F: FnMut() -> T,
{
    let start = Instant::now();
    while start.elapsed() < timeout {
        if check() == expected {
            return;
        }
        sleep(Duration::from_millis(50));
    }
    panic!("timeout");
}

static TRACING_INITIALIZED: OnceLock<()> = OnceLock::new();

fn init_tracing() {
    TRACING_INITIALIZED.get_or_init(|| {
        let dirs = std::env::var(EnvFilter::DEFAULT_ENV).unwrap_or(String::from("off"));
        let filter = EnvFilter::builder().parse_lossy(dirs);

        tracing_subscriber::fmt::fmt()
            .with_env_filter(filter)
            .with_ansi(true)
            .init();
    });
}

pub(crate) fn establish_tcp(node: &Node, peer: &Node) {
    node.peer_connector
        .connect_to(peer.tcp_listener.local_address());

    assert_timely(
        Duration::from_secs(2),
        || {
            node.network
                .find_node_id(&peer.node_id.public_key())
                .is_some()
        },
        "node did not connect",
    )
}
