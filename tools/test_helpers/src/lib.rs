use rsnano_core::{
    work::WorkPoolImpl, Account, Amount, Block, BlockHash, Epoch, KeyPair, Networks, PublicKey,
    StateBlock, StateBlockBuilder, WalletId, DEV_GENESIS_KEY,
};
use rsnano_ledger::{BlockStatus, DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
use rsnano_network::{Channel, ChannelDirection, ChannelInfo, ChannelMode};
use rsnano_node::{
    block_processing::BacklogPopulationConfig,
    config::{NodeConfig, NodeFlags, WebsocketConfig},
    consensus::{ActiveElectionsExt, Election},
    unique_path,
    utils::AsyncRuntime,
    wallets::WalletsExt,
    NetworkParams, Node, NodeBuilder, NodeExt,
};
use rsnano_nullable_tcp::TcpStream;
use rsnano_rpc_client::{NanoRpcClient, Url};
use rsnano_rpc_server::run_rpc_server;
use rsnano_websocket_server::{create_websocket_server, WebsocketListener, WebsocketListenerExt};
use std::{
    net::{IpAddr, Ipv6Addr, SocketAddr, TcpListener},
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
    pub nodes: Vec<Arc<Node>>,
    pub initialization_blocks: Vec<Block>,
    pub initialization_blocks_cemented: Vec<Block>,
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
            initialization_blocks: Vec::new(),
            initialization_blocks_cemented: Vec::new(),
        }
    }

    pub fn default_config() -> NodeConfig {
        let network_params = NetworkParams::new(Networks::NanoDevNetwork);
        let port = get_available_port();
        let mut config = NodeConfig::new(Some(port), &network_params, 1);
        config.representative_vote_weight_minimum = Amount::zero();
        config
    }

    pub fn default_config_without_backlog_population() -> NodeConfig {
        NodeConfig {
            backlog: BacklogPopulationConfig {
                enabled: false,
                ..Default::default()
            },
            ..Self::default_config()
        }
    }

    pub fn build_node<'a>(&'a mut self) -> TestNodeBuilder<'a> {
        TestNodeBuilder {
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

    fn setup_node(&mut self, node: &Node) {
        let mut tx = node.store.tx_begin_write();
        for block in &mut self.initialization_blocks {
            node.ledger.process(&mut tx, block).unwrap();
        }

        for block in &mut self.initialization_blocks_cemented {
            node.ledger.process(&mut tx, block).unwrap();
            node.ledger.confirm(&mut tx, block.hash());
        }
    }

    fn make_node_with(
        &mut self,
        config: NodeConfig,
        flags: NodeFlags,
        disconnected: bool,
    ) -> Arc<Node> {
        let node = self.new_node(config, flags);

        self.setup_node(&node);

        let wallet_id = WalletId::random();
        node.wallets.create(wallet_id);
        node.start();

        // Check that we don't start more nodes than limit for single IP address
        debug_assert!(
            self.nodes.len() < node.config.max_peers_per_ip.into()
                || node.flags.disable_max_peers_per_ip
        );
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

    fn make_node_with_websocket_server(
        &mut self,
        config: NodeConfig,
        flags: NodeFlags,
        disconnected: bool,
    ) -> (Arc<Node>, Arc<WebsocketListener>) {
        let node = self.new_node_with_websocket_server(config, flags);

        self.setup_node(&node);

        let wallet_id = WalletId::random();
        node.wallets.create(wallet_id);

        let websocket_server = create_websocket_server(
            WebsocketConfig {
                enabled: node.config.websocket_config.enabled,
                port: node.config.websocket_config.port,
                address: node.config.websocket_config.address.clone(),
            },
            node.wallets.clone(),
            node.runtime.clone(),
            &node.active_elections,
            &node.telemetry,
            &node.vote_processor,
            &node.process_live_dispatcher,
            &node.bootstrap_initiator,
        )
        .unwrap();

        websocket_server.start();

        let node = Arc::new(node);

        node.start();

        // Check that we don't start more nodes than limit for single IP address
        debug_assert!(
            self.nodes.len() < node.config.max_peers_per_ip.into()
                || node.flags.disable_max_peers_per_ip
        );
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

        (node, websocket_server)
    }

    fn new_node(&self, config: NodeConfig, flags: NodeFlags) -> Arc<Node> {
        let path = unique_path().expect("Could not get a unique path");
        let node = NodeBuilder::new(self.network_params.network.current_network)
            .runtime(self.runtime.tokio.handle().clone())
            .data_path(path)
            .config(config)
            .network_params(self.network_params.clone())
            .flags(flags)
            .work(self.work.clone())
            .finish()
            .unwrap();
        Arc::new(node)
    }

    fn new_node_with_websocket_server(&self, config: NodeConfig, flags: NodeFlags) -> Node {
        let path = unique_path().expect("Could not get a unique path");
        let node = NodeBuilder::new(self.network_params.network.current_network)
            .runtime(self.runtime.tokio.handle().clone())
            .data_path(path)
            .config(config)
            .network_params(self.network_params.clone())
            .flags(flags)
            .work(self.work.clone())
            .finish()
            .unwrap();
        node
    }

    fn stop(&mut self) {
        for node in &self.nodes {
            node.stop();
            std::fs::remove_dir_all(&node.data_path).expect("Could not delete node data dir");
        }
        self.work.stop();
    }
}

impl Drop for System {
    fn drop(&mut self) {
        self.stop();
    }
}

pub struct TestNodeBuilder<'a> {
    system: &'a mut System,
    config: Option<NodeConfig>,
    flags: Option<NodeFlags>,
    disconnected: bool,
}

impl<'a> TestNodeBuilder<'a> {
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

    pub fn finish_with_websocket_server(self) -> (Arc<Node>, Arc<WebsocketListener>) {
        let config = self.config.unwrap_or_else(|| System::default_config());
        let flags = self.flags.unwrap_or_default();
        self.system
            .make_node_with_websocket_server(config, flags, self.disconnected)
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
    node.network
        .add(
            TcpStream::new_null(),
            ChannelDirection::Inbound,
            ChannelMode::Realtime,
        )
        .unwrap()
}

pub fn start_election(node: &Node, hash: &BlockHash) -> Arc<Election> {
    assert_timely_msg(
        Duration::from_secs(5),
        || node.block_exists(hash),
        "block not in ledger",
    );

    let block = node.block(hash).unwrap();
    node.election_schedulers.add_manual(block.clone());
    // wait for the election to appear
    assert_timely_msg(
        Duration::from_secs(5),
        || {
            node.active_elections
                .election(&block.qualified_root())
                .is_some()
        },
        "election not active",
    );
    let election = node
        .active_elections
        .election(&block.qualified_root())
        .unwrap();
    election.transition_active();
    election
}

pub fn start_elections(node: &Node, hashes: &[BlockHash], forced: bool) {
    for hash in hashes {
        let election = start_election(node, hash);
        if forced {
            node.active_elections.force_confirm(&election);
        }
    }
}

pub fn activate_hashes(node: &Node, hashes: &[BlockHash]) {
    for hash in hashes {
        let block = node.block(hash).unwrap();
        node.election_schedulers.add_manual(block);
    }
}

pub fn setup_chain(node: &Node, count: usize, target: &KeyPair, confirm: bool) -> Vec<Block> {
    let mut latest = node.latest(&target.account());
    let mut balance = node.balance(&target.account());

    let mut blocks = Vec::new();

    for _ in 0..count {
        let throwaway = KeyPair::new();
        balance = balance - Amount::raw(1);
        let send = Block::State(StateBlock::new(
            target.account(),
            latest,
            target.public_key(),
            balance,
            throwaway.account().into(),
            &target,
            node.work_generate_dev(latest),
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
) -> Vec<(Account, Vec<Block>)> {
    let mut latest = node.latest(&source.account());
    let mut balance = node.balance(&source.account());

    let mut chains = Vec::new();
    for _ in 0..chain_count {
        let key = KeyPair::new();
        let amount_sent = Amount::raw(block_count as u128 * 2);
        balance = balance - amount_sent; // Send enough to later create `block_count` blocks
        let send = Block::State(StateBlock::new(
            source.account(),
            latest,
            source.public_key(),
            balance,
            key.account().into(),
            source,
            node.work_generate_dev(latest),
        ));

        let open = Block::State(StateBlock::new(
            key.account(),
            BlockHash::zero(),
            key.public_key(),
            amount_sent,
            send.hash().into(),
            &key,
            node.work_generate_dev(&key),
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

pub fn setup_independent_blocks(node: &Node, count: usize, source: &KeyPair) -> Vec<Block> {
    let mut blocks = Vec::new();
    let account: Account = source.public_key().into();
    let mut latest = node.latest(&account);
    let mut balance = node.balance(&account);

    for _ in 0..count {
        let key = KeyPair::new();

        balance -= 1;

        let send = Block::State(StateBlock::new(
            account,
            latest,
            source.public_key(),
            balance,
            key.public_key().as_account().into(),
            source,
            node.work_generate_dev(latest),
        ));

        latest = send.hash();

        let open = Block::State(StateBlock::new(
            key.public_key().into(),
            BlockHash::zero(),
            key.public_key(),
            Amount::raw(1),
            send.hash().into(),
            &key,
            node.work_generate_dev(&key),
        ));

        node.process_multi(&[send.clone(), open.clone()]);
        // Ensure blocks are in the ledger
        assert_timely(Duration::from_secs(5), || {
            node.block_hashes_exist([send.hash(), open.hash()])
        });

        blocks.push(open);
    }

    // Confirm whole genesis chain at once
    node.confirm(latest);

    blocks
}

use tokio::net::TcpListener as TokioTcpListener;

pub struct RpcServerGuard {
    pub client: Arc<NanoRpcClient>,
    tx_stop: Option<tokio::sync::oneshot::Sender<()>>,
}

impl Drop for RpcServerGuard {
    fn drop(&mut self) {
        if let Some(tx) = self.tx_stop.take() {
            let _ = tx.send(());
        }
    }
}

pub fn setup_rpc_client_and_server(node: Arc<Node>, enable_control: bool) -> RpcServerGuard {
    let port = get_available_port();
    let socket_addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), port);
    let rpc_url = format!("http://[::1]:{}/", port);
    let rpc_client = Arc::new(NanoRpcClient::new(Url::parse(&rpc_url).unwrap()));

    let listener = node.runtime.block_on(async {
        TokioTcpListener::bind(socket_addr)
            .await
            .expect("Failed to bind to address")
    });

    let (tx_stop, rx_stop) = tokio::sync::oneshot::channel();
    let (tx_stop2, rx_stop2) = tokio::sync::oneshot::channel();

    node.runtime.spawn(run_rpc_server(
        node.clone(),
        listener,
        enable_control,
        tx_stop,
        async move {
            tokio::select! {
                _ = rx_stop => {},
                _ = rx_stop2 => {}
            }
        },
    ));

    RpcServerGuard {
        client: rpc_client,
        tx_stop: Some(tx_stop2),
    }
}

pub fn send_block(node: Arc<Node>) -> BlockHash {
    let send1 = send_block_to(node, *DEV_GENESIS_ACCOUNT, Amount::raw(1));
    send1.hash()
}

pub fn send_block_to(node: Arc<Node>, account: Account, amount: Amount) -> Block {
    let transaction = node.ledger.read_txn();

    let previous = node
        .ledger
        .any()
        .account_head(&transaction, &*DEV_GENESIS_ACCOUNT)
        .unwrap_or(*DEV_GENESIS_HASH);

    let balance = node
        .ledger
        .any()
        .account_balance(&transaction, &*DEV_GENESIS_ACCOUNT)
        .unwrap_or(Amount::MAX);

    let send = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        previous,
        *DEV_GENESIS_PUB_KEY,
        balance - amount,
        account.into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(previous),
    ));

    node.process_active(send.clone());
    assert_timely_msg(
        Duration::from_secs(5),
        || node.active_elections.active(&send),
        "not active on node",
    );

    send
}

pub fn process_block_local(node: Arc<Node>, account: Account, amount: Amount) -> Block {
    let transaction = node.ledger.read_txn();

    let previous = node
        .ledger
        .any()
        .account_head(&transaction, &*DEV_GENESIS_ACCOUNT)
        .unwrap_or(*DEV_GENESIS_HASH);

    let balance = node
        .ledger
        .any()
        .account_balance(&transaction, &*DEV_GENESIS_ACCOUNT)
        .unwrap_or(Amount::MAX);

    let send = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        previous,
        *DEV_GENESIS_PUB_KEY,
        balance - amount,
        account.into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(previous),
    ));

    node.process_local(send.clone()).unwrap();

    send
}

pub fn process_send_block(node: Arc<Node>, account: Account, amount: Amount) -> Block {
    let transaction = node.ledger.read_txn();

    let previous = node
        .ledger
        .any()
        .account_head(&transaction, &*DEV_GENESIS_ACCOUNT)
        .unwrap_or(*DEV_GENESIS_HASH);

    let balance = node
        .ledger
        .any()
        .account_balance(&transaction, &*DEV_GENESIS_ACCOUNT)
        .unwrap_or(Amount::MAX);

    let send = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        previous,
        *DEV_GENESIS_PUB_KEY,
        balance - amount,
        account.into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(previous),
    ));

    node.process(send.clone()).unwrap();

    send
}

pub fn process_open_block(node: Arc<Node>, keys: KeyPair) -> Block {
    let transaction = node.ledger.read_txn();
    let account = keys.account();

    let (key, info) = node
        .ledger
        .any()
        .account_receivable_upper_bound(&transaction, account, BlockHash::zero())
        .next()
        .unwrap();

    let open = Block::State(StateBlock::new(
        account,
        BlockHash::zero(),
        keys.public_key(),
        info.amount,
        key.send_block_hash.into(),
        &keys,
        node.work_generate_dev(account),
    ));

    node.process(open.clone()).unwrap();

    open
}

pub fn upgrade_epoch(
    node: Arc<Node>,
    //pool: &mut WorkPoolImpl,
    epoch: Epoch,
) -> Block {
    let transaction = node.ledger.read_txn();
    let account = *DEV_GENESIS_ACCOUNT;
    let latest = node
        .ledger
        .any()
        .account_head(&transaction, &account)
        .unwrap();
    let balance = node
        .ledger
        .any()
        .account_balance(&transaction, &account)
        .unwrap_or(Amount::zero());

    let builder = StateBlockBuilder::new();
    let epoch_block = builder
        .account(account)
        .previous(latest)
        .balance(balance)
        .link(node.ledger.epoch_link(epoch).unwrap())
        .representative(*DEV_GENESIS_PUB_KEY)
        .sign(&DEV_GENESIS_KEY)
        .work(node.work_generate_dev(*DEV_GENESIS_HASH))
        .build();

    assert_eq!(
        BlockStatus::Progress,
        node.process_local(epoch_block.clone()).unwrap()
    );

    epoch_block
}

pub fn setup_new_account(
    node: &Node,
    amount: Amount,
    source: &KeyPair,
    dest: &KeyPair,
    dest_rep: PublicKey,
    force_confirm: bool,
) -> (Block, Block) {
    let source_account = source.public_key().as_account();
    let dest_account = dest.public_key().as_account();
    let latest = node.latest(&source_account);
    let balance = node.balance(&source_account);

    let send = Block::State(StateBlock::new(
        source_account,
        latest,
        source.public_key(),
        balance - amount,
        dest_account.into(),
        source,
        node.work_generate_dev(latest),
    ));

    let open = Block::State(StateBlock::new(
        dest_account,
        BlockHash::zero(),
        dest_rep,
        amount,
        send.hash().into(),
        dest,
        node.work_generate_dev(dest_account),
    ));

    node.process(send.clone()).unwrap();
    node.process(open.clone()).unwrap();

    if force_confirm {
        node.confirm(send.hash());
        node.confirm(open.hash());
    }

    (send, open)
}
