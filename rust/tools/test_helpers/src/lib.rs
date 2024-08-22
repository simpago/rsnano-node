use rsnano_core::{
    work::WorkPoolImpl, Account, Amount, BlockEnum, BlockHash, KeyPair, Networks, StateBlock,
    WalletId,
};
use rsnano_node::{
    config::{NodeConfig, NodeFlags},
    consensus::{ActiveElectionsExt, Election},
    node::{Node, NodeExt},
    transport::{Channel, ChannelDirection, ChannelInfo, ChannelMode, PeerConnectorExt},
    unique_path,
    utils::AsyncRuntime,
    wallets::WalletsExt,
    NetworkParams,
};
use rsnano_nullable_tcp::TcpStream;
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

pub struct System {
    runtime: Arc<AsyncRuntime>,
    network_params: NetworkParams,
    pub work: Arc<WorkPoolImpl>,
    nodes: Vec<Arc<Node>>,
}

impl System {
    pub fn new() -> Self {
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

    pub fn default_config() -> NodeConfig {
        let network_params = NetworkParams::new(Networks::NanoDevNetwork);
        let port = get_available_port();
        let mut config = NodeConfig::new(Some(port), &network_params, 1);
        config.representative_vote_weight_minimum = Amount::zero();
        config
    }

    pub fn build_node<'a>(&'a mut self) -> NodeBuilder<'a> {
        NodeBuilder {
            system: self,
            config: None,
            flags: None,
            disconnected: false,
        }
    }

    pub fn make_disconnected_node(&mut self) -> Arc<Node> {
        self.build_node().disconnected().finish()
    }

    pub fn make_node(&mut self) -> Arc<Node> {
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
            let other = &self.nodes[0];
            other
                .peer_connector
                .connect_to(node.tcp_listener.local_address());

            let start = Instant::now();
            loop {
                if node
                    .network_info
                    .read()
                    .unwrap()
                    .find_node_id(&other.node_id.public_key())
                    .is_some()
                    && other
                        .network_info
                        .read()
                        .unwrap()
                        .find_node_id(&node.node_id.public_key())
                        .is_some()
                {
                    break;
                }

                if start.elapsed() > Duration::from_secs(5) {
                    panic!("connection not successfull");
                }
                sleep(Duration::from_millis(10));
            }
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

pub struct NodeBuilder<'a> {
    system: &'a mut System,
    config: Option<NodeConfig>,
    flags: Option<NodeFlags>,
    disconnected: bool,
}

impl<'a> NodeBuilder<'a> {
    pub fn config(mut self, cfg: NodeConfig) -> Self {
        self.config = Some(cfg);
        self
    }

    pub fn flags(mut self, flags: NodeFlags) -> Self {
        self.flags = Some(flags);
        self
    }

    pub fn disconnected(mut self) -> Self {
        self.disconnected = true;
        self
    }

    pub fn finish(self) -> Arc<Node> {
        let config = self.config.unwrap_or_else(|| System::default_config());
        let flags = self.flags.unwrap_or_default();
        self.system.make_node_with(config, flags, self.disconnected)
    }
}

static START_PORT: AtomicU16 = AtomicU16::new(1025);

pub fn get_available_port() -> u16 {
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

pub fn assert_never(duration: Duration, mut check: impl FnMut() -> bool) {
    let start = Instant::now();
    while start.elapsed() < duration {
        if check() {
            panic!("never check failed");
        }
        sleep(Duration::from_millis(50));
    }
}

pub fn assert_timely<F>(timeout: Duration, check: F)
where
    F: FnMut() -> bool,
{
    assert_timely_msg(timeout, check, "timeout");
}

pub fn assert_timely_msg<F>(timeout: Duration, mut check: F, error_message: &str)
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

pub fn assert_timely_eq<T, F>(timeout: Duration, mut check: F, expected: T)
where
    T: PartialEq + std::fmt::Debug + Clone,
    F: FnMut() -> T,
{
    let start = Instant::now();
    let mut actual = expected.clone();
    while start.elapsed() < timeout {
        actual = check();
        if actual == expected {
            return;
        }
        sleep(Duration::from_millis(50));
    }
    panic!("timeout. expected: {expected:?}, actual: {actual:?}");
}

pub fn assert_always_eq<T, F>(time: Duration, mut condition: F, expected: T)
where
    T: PartialEq + std::fmt::Debug,
    F: FnMut() -> T,
{
    let start = Instant::now();
    while start.elapsed() < time {
        assert_eq!(condition(), expected);
        sleep(Duration::from_millis(50));
    }
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

pub fn establish_tcp(node: &Node, peer: &Node) -> Arc<ChannelInfo> {
    node.peer_connector
        .connect_to(peer.tcp_listener.local_address());

    assert_timely_msg(
        Duration::from_secs(2),
        || {
            node.network_info
                .read()
                .unwrap()
                .find_node_id(&peer.node_id.public_key())
                .is_some()
        },
        "node did not connect",
    );

    node.network_info
        .read()
        .unwrap()
        .find_node_id(&peer.node_id.public_key())
        .unwrap()
        .clone()
}

pub fn make_fake_channel(node: &Node) -> Arc<Channel> {
    node.async_rt
        .tokio
        .block_on(node.network.add(
            TcpStream::new_null(),
            ChannelDirection::Inbound,
            ChannelMode::Realtime,
        ))
        .unwrap()
}

pub fn start_election(node: &Node, hash: &BlockHash) -> Arc<Election> {
    assert_timely_msg(
        Duration::from_secs(5),
        || node.block_exists(hash),
        "block not in ledger",
    );

    let block = node.block(hash).unwrap();
    node.manual_scheduler.push(Arc::new(block.clone()), None);
    // wait for the election to appear
    assert_timely_msg(
        Duration::from_secs(5),
        || node.active.election(&block.qualified_root()).is_some(),
        "election not active",
    );
    let election = node.active.election(&block.qualified_root()).unwrap();
    election.transition_active();
    election
}

pub fn start_elections(node: &Node, hashes: &[BlockHash], forced: bool) {
    for hash in hashes {
        let election = start_election(node, hash);
        if forced {
            node.active.force_confirm(&election);
        }
    }
}

pub fn activate_hashes(node: &Node, hashes: &[BlockHash]) {
    for hash in hashes {
        let block = node.block(hash).unwrap();
        node.manual_scheduler.push(Arc::new(block), None);
    }
}

pub fn setup_chain(node: &Node, count: usize, target: &KeyPair, confirm: bool) -> Vec<BlockEnum> {
    let mut latest = node.latest(&target.account());
    let mut balance = node.balance(&target.account());

    let mut blocks = Vec::new();

    for _ in 0..count {
        let throwaway = KeyPair::new();
        balance = balance - Amount::raw(1);
        let send = BlockEnum::State(StateBlock::new(
            target.account(),
            latest,
            target.public_key(),
            balance,
            throwaway.account().into(),
            &target,
            node.work_generate_dev(latest.into()),
        ));
        latest = send.hash();
        blocks.push(send);
    }

    for block in &blocks {
        node.process(block.clone()).unwrap();
    }

    if confirm {
        // Confirm whole chain at once
        for block in &blocks {
            node.confirm(block.hash());
        }
    }

    blocks
}

pub fn setup_chains(
    node: &Node,
    chain_count: usize,
    block_count: usize,
    source: &KeyPair,
    confirm: bool,
) -> Vec<(Account, Vec<BlockEnum>)> {
    let mut latest = node.latest(&source.account());
    let mut balance = node.balance(&source.account());

    let mut chains = Vec::new();
    for _ in 0..chain_count {
        let key = KeyPair::new();
        let amount_sent = Amount::raw(block_count as u128 * 2);
        balance = balance - amount_sent; // Send enough to later create `block_count` blocks
        let send = BlockEnum::State(StateBlock::new(
            source.account(),
            latest,
            source.public_key(),
            balance,
            key.account().into(),
            source,
            node.work_generate_dev(latest.into()),
        ));

        let open = BlockEnum::State(StateBlock::new(
            key.account(),
            BlockHash::zero(),
            key.public_key(),
            amount_sent,
            send.hash().into(),
            &key,
            node.work_generate_dev(key.public_key().into()),
        ));

        latest = send.hash();
        node.process(send.clone()).unwrap();
        node.process(open.clone()).unwrap();

        if confirm {
            node.confirm(send.hash());
            node.confirm(open.hash());
        }

        let mut blocks = setup_chain(node, block_count, &key, confirm);
        blocks.insert(0, open);

        chains.push((key.account(), blocks));
    }

    chains
}
