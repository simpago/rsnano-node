mod bootstrap_attempt;
mod bootstrap_attempts;
mod bootstrap_client;
mod bootstrap_initiator;
mod bootstrap_lazy;
mod channel_tcp_wrapper;
mod pulls_cache;

pub(crate) use bootstrap_attempt::*;
pub(crate) use bootstrap_initiator::*;

pub use bootstrap_client::{
    BootstrapClient, BootstrapClientObserver, BootstrapClientObserverWeakPtr,
};

pub use bootstrap_attempts::BootstrapAttempts;
pub use bootstrap_lazy::BootstrapAttemptLazy;
pub use channel_tcp_wrapper::ChannelTcpWrapper;
pub use pulls_cache::{PullInfo, PullsCache};

pub mod bootstrap_limits {
    pub const PULL_COUNT_PER_CHECK: u64 = 8 * 1024;
    pub const BOOTSTRAP_MINIMUM_ELAPSED_SECONDS_BLOCKRATE: f64 = 0.02;
}

#[derive(Clone, Copy, FromPrimitive)]
pub enum BootstrapMode {
    Legacy,
    Lazy,
    WalletLazy,
}

pub enum BootstrapStrategy {
    Lazy(BootstrapAttemptLazy),
    Other(BootstrapAttempt),
}

impl BootstrapStrategy {
    pub fn attempt(&self) -> &BootstrapAttempt {
        match self {
            BootstrapStrategy::Other(i) => i,
            BootstrapStrategy::Lazy(i) => &i.attempt,
        }
    }
}
