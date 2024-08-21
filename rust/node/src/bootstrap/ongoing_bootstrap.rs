use super::{BootstrapInitiator, BootstrapInitiatorExt};
use crate::{
    config::NodeFlags,
    stats::{DetailType, Direction, StatType, Stats},
    transport::{ChannelMode, NetworkInfo},
    utils::ThreadPool,
    NetworkParams,
};
use rsnano_core::Account;
use rsnano_ledger::Ledger;
use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, RwLock,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub struct OngoingBootstrap {
    network_params: NetworkParams,
    warmed_up: AtomicU32,
    bootstrap_initiator: Arc<BootstrapInitiator>,
    network: Arc<RwLock<NetworkInfo>>,
    flags: NodeFlags,
    ledger: Arc<Ledger>,
    stats: Arc<Stats>,
    workers: Arc<dyn ThreadPool>,
}

impl OngoingBootstrap {
    pub fn new(
        network_params: NetworkParams,
        bootstrap_initiator: Arc<BootstrapInitiator>,
        network: Arc<RwLock<NetworkInfo>>,
        flags: NodeFlags,
        ledger: Arc<Ledger>,
        stats: Arc<Stats>,
        workers: Arc<dyn ThreadPool>,
    ) -> Self {
        Self {
            network_params,
            warmed_up: AtomicU32::new(0),
            bootstrap_initiator,
            network,
            flags,
            ledger,
            stats,
            workers,
        }
    }
}

pub trait OngoingBootstrapExt {
    fn ongoing_bootstrap(&self);
}

impl OngoingBootstrapExt for Arc<OngoingBootstrap> {
    fn ongoing_bootstrap(&self) {
        let mut next_wakeup =
            Duration::from_secs(self.network_params.network.bootstrap_interval_s as u64);
        if self.warmed_up.load(Ordering::SeqCst) < 3 {
            // Re-attempt bootstrapping more aggressively on startup
            next_wakeup = Duration::from_secs(5);
            if !self.bootstrap_initiator.in_progress()
                && !self
                    .network
                    .read()
                    .unwrap()
                    .count_by_mode(ChannelMode::Realtime)
                    == 0
            {
                self.warmed_up.fetch_add(1, Ordering::SeqCst);
            }
        }
        if self.network_params.network.is_dev_network() && self.flags.bootstrap_interval != 0 {
            // For test purposes allow faster automatic bootstraps
            next_wakeup = Duration::from_secs(self.flags.bootstrap_interval as u64);
            self.warmed_up.fetch_add(1, Ordering::SeqCst);
        }
        // Differential bootstrap with max age (75% of all legacy attempts)
        let mut frontiers_age = u32::MAX;
        let bootstrap_weight_reached =
            self.ledger.block_count() >= self.ledger.bootstrap_weight_max_blocks();
        let previous_bootstrap_count =
            self.stats
                .count(StatType::Bootstrap, DetailType::Initiate, Direction::Out)
                + self.stats.count(
                    StatType::Bootstrap,
                    DetailType::InitiateLegacyAge,
                    Direction::Out,
                );
        /*
        - Maximum value for 25% of attempts or if block count is below preconfigured value (initial bootstrap not finished)
        - Node shutdown time minus 1 hour for start attempts (warm up)
        - Default age value otherwise (1 day for live network, 1 hour for beta)
        */
        if bootstrap_weight_reached {
            if self.warmed_up.load(Ordering::SeqCst) < 3 {
                // Find last online weight sample (last active time for node)
                let mut last_sample_time = UNIX_EPOCH;

                {
                    let tx = self.ledger.read_txn();
                    let last_record = self.ledger.store.online_weight.rbegin(&tx);
                    if let Some(last_record) = last_record.current() {
                        last_sample_time = UNIX_EPOCH
                            .checked_add(Duration::from_nanos(*last_record.0))
                            .unwrap();
                    }
                }

                let time_since_last_sample = SystemTime::now()
                    .duration_since(last_sample_time)
                    .unwrap_or(Duration::MAX);

                if time_since_last_sample.as_secs() + 60 * 60 < u32::MAX as u64 {
                    frontiers_age = std::cmp::max(
                        (time_since_last_sample.as_secs() + 60 * 60) as u32,
                        self.network_params.bootstrap.default_frontiers_age_seconds,
                    );
                }
            } else if previous_bootstrap_count % 4 != 0 {
                frontiers_age = self.network_params.bootstrap.default_frontiers_age_seconds;
            }
        }
        // Bootstrap and schedule for next attempt
        self.bootstrap_initiator.bootstrap(
            false,
            format!("auto_bootstrap_{}", previous_bootstrap_count),
            frontiers_age,
            Account::zero(),
        );
        let self_w = Arc::downgrade(self);
        self.workers.add_delayed_task(
            next_wakeup,
            Box::new(move || {
                if let Some(node) = self_w.upgrade() {
                    node.ongoing_bootstrap();
                }
            }),
        );
    }
}
