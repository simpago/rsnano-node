mod ascending;
mod bootstrap_attempt;
mod bootstrap_attempts;
mod bootstrap_client;
mod bootstrap_connections;
mod bootstrap_initiator;
mod bootstrap_lazy;
mod bootstrap_legacy;
mod bootstrap_message_visitor;
mod bootstrap_message_visitor_factory;
mod bootstrap_server;
mod bootstrap_wallet;
mod bulk_pull_account_client;
mod bulk_pull_account_server;
mod bulk_pull_client;
mod bulk_pull_server;
mod bulk_push_client;
mod bulk_push_server;
mod frontier_req_client;
mod frontier_req_server;
mod ongoing_bootstrap;
mod pulls_cache;

pub use ascending::*;
pub use bootstrap_attempt::*;
pub use bootstrap_attempts::BootstrapAttempts;
pub use bootstrap_client::BootstrapClient;
pub use bootstrap_connections::*;
pub use bootstrap_initiator::*;
pub use bootstrap_lazy::*;
pub use bootstrap_legacy::*;
pub use bootstrap_message_visitor::BootstrapMessageVisitorImpl;
pub use bootstrap_message_visitor_factory::BootstrapMessageVisitorFactory;
pub use bootstrap_server::*;
pub use bootstrap_wallet::*;
pub use bulk_pull_account_client::*;
pub use bulk_pull_account_server::BulkPullAccountServer;
pub use bulk_pull_client::*;
pub use bulk_pull_server::BulkPullServer;
pub use bulk_push_client::*;
pub use bulk_push_server::BulkPushServer;
pub use frontier_req_client::*;
pub use frontier_req_server::FrontierReqServer;
pub use ongoing_bootstrap::*;
pub use pulls_cache::{PullInfo, PullsCache};
use rsnano_core::{utils::PropertyTree, Account, BlockEnum};
use std::{ops::Deref, sync::Arc};

pub mod bootstrap_limits {
    pub const PULL_COUNT_PER_CHECK: u64 = 8 * 1024;
    pub const LAZY_BLOCKS_RESTART_LIMIT: usize = 1024 * 1024;
    pub const BOOTSTRAP_CONNECTION_SCALE_TARGET_BLOCKS: u32 = 10000;
    pub const BOOTSTRAP_CONNECTION_WARMUP_TIME_SEC: f64 = 5.0;
    pub const BOOTSTRAP_MINIMUM_ELAPSED_SECONDS_BLOCKRATE: f64 = 0.02;
    pub const BOOTSTRAP_MINIMUM_FRONTIER_BLOCKS_PER_SEC: f64 = 1000.0;
    pub const BOOTSTRAP_MINIMUM_BLOCKS_PER_SEC: f64 = 10.0;
    pub const BOOTSTRAP_MINIMUM_TERMINATION_TIME_SEC: f64 = 30.0;
    pub const BOOTSTRAP_MAX_NEW_CONNECTIONS: u32 = 32;
    pub const REQUEUED_PULLS_PROCESSED_BLOCKS_FACTOR: u32 = 4096;
    pub const LAZY_BATCH_PULL_COUNT_RESIZE_BLOCKS_LIMIT: u64 = 4 * 1024 * 1024;
    pub const LAZY_BATCH_PULL_COUNT_RESIZE_RATIO: f64 = 2.0;
    pub const BULK_PUSH_COST_LIMIT: u64 = 200;
}

#[derive(Clone, Copy, FromPrimitive, Debug, PartialEq, Eq)]
pub enum BootstrapMode {
    Legacy,
    Lazy,
    WalletLazy,
    Ascending,
}

impl BootstrapMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            BootstrapMode::Legacy => "legacy",
            BootstrapMode::Lazy => "lazy",
            BootstrapMode::WalletLazy => "wallet_lazy",
            BootstrapMode::Ascending => "ascending",
        }
    }
}

pub enum BootstrapStrategy {
    Lazy(BootstrapAttemptLazy),
    Legacy(Arc<BootstrapAttemptLegacy>),
    Wallet(Arc<BootstrapAttemptWallet>),
}

impl Deref for BootstrapStrategy {
    type Target = dyn BootstrapAttemptTrait;

    fn deref(&self) -> &Self::Target {
        match self {
            BootstrapStrategy::Lazy(i) => i,
            BootstrapStrategy::Legacy(i) => i.as_ref(),
            BootstrapStrategy::Wallet(i) => i.as_ref(),
        }
    }
}

impl BootstrapStrategy {
    pub fn mode(&self) -> BootstrapMode {
        match self {
            BootstrapStrategy::Lazy(_) => BootstrapMode::Lazy,
            BootstrapStrategy::Legacy(_) => BootstrapMode::Legacy,
            BootstrapStrategy::Wallet(_) => BootstrapMode::WalletLazy,
        }
    }

    pub fn attempt(&self) -> &BootstrapAttempt {
        match self {
            BootstrapStrategy::Lazy(i) => &i.attempt,
            BootstrapStrategy::Legacy(i) => &i.attempt,
            BootstrapStrategy::Wallet(i) => &i.attempt,
        }
    }

    pub fn run(&self) {
        match self {
            BootstrapStrategy::Lazy(i) => i.run(),
            BootstrapStrategy::Legacy(i) => i.run(),
            BootstrapStrategy::Wallet(i) => i.run(),
        }
    }

    pub fn stop(&self) {
        match self {
            BootstrapStrategy::Legacy(i) => i.stop(),
            BootstrapStrategy::Lazy(i) => i.attempt.stop(),
            BootstrapStrategy::Wallet(i) => i.attempt.stop(),
        }
    }

    pub fn get_information(&self, tree: &mut dyn PropertyTree) {
        match self {
            BootstrapStrategy::Lazy(i) => i.get_information(tree).unwrap(),
            BootstrapStrategy::Legacy(i) => i.get_information(tree),
            BootstrapStrategy::Wallet(i) => i.get_information(tree),
        }
    }

    pub fn process_block(
        &self,
        block: Arc<BlockEnum>,
        known_account: &Account,
        pull_blocks_processed: u64,
        max_blocks: u32,
        block_expected: bool,
        retry_limit: u32,
    ) -> bool {
        match self {
            BootstrapStrategy::Legacy(i) => i.process_block(block, pull_blocks_processed),
            BootstrapStrategy::Lazy(i) => i.process_block(
                block,
                known_account,
                pull_blocks_processed,
                max_blocks,
                block_expected,
                retry_limit,
            ),
            BootstrapStrategy::Wallet(i) => i.process_block(block, pull_blocks_processed),
        }
    }
}
