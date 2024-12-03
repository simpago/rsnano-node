use crate::{
    cementation::ConfirmingSet,
    config::{NetworkConstants, NodeConfig},
    representatives::OnlineReps,
    stats::Stats,
};

use super::{
    ActiveElections, HintedScheduler, HintedSchedulerExt, ManualScheduler, ManualSchedulerExt,
    OptimisticScheduler, OptimisticSchedulerExt, PriorityScheduler, PrioritySchedulerExt,
    VoteCache,
};
use rsnano_core::{utils::ContainerInfo, Account, AccountInfo, ConfirmationHeightInfo, SavedBlock};
use rsnano_ledger::Ledger;
use rsnano_output_tracker::{OutputListenerMt, OutputTrackerMt};
use rsnano_store_lmdb::{LmdbReadTransaction, Transaction};
use std::sync::{Arc, Mutex};

pub struct ElectionSchedulers {
    pub priority: Arc<PriorityScheduler>,
    optimistic: Arc<OptimisticScheduler>,
    hinted: Arc<HintedScheduler>,
    pub manual: Arc<ManualScheduler>,
    notify_listener: OutputListenerMt<()>,
}

impl ElectionSchedulers {
    pub fn new(
        config: &NodeConfig,
        network_constants: NetworkConstants,
        active_elections: Arc<ActiveElections>,
        ledger: Arc<Ledger>,
        stats: Arc<Stats>,
        vote_cache: Arc<Mutex<VoteCache>>,
        confirming_set: Arc<ConfirmingSet>,
        online_reps: Arc<Mutex<OnlineReps>>,
    ) -> Self {
        let hinted = Arc::new(HintedScheduler::new(
            config.hinted_scheduler.clone(),
            active_elections.clone(),
            ledger.clone(),
            stats.clone(),
            vote_cache.clone(),
            confirming_set.clone(),
            online_reps.clone(),
        ));

        let manual = Arc::new(ManualScheduler::new(
            stats.clone(),
            active_elections.clone(),
        ));

        let optimistic = Arc::new(OptimisticScheduler::new(
            config.optimistic_scheduler.clone(),
            stats.clone(),
            active_elections.clone(),
            network_constants,
            ledger.clone(),
            confirming_set.clone(),
        ));

        let priority = Arc::new(PriorityScheduler::new(
            config.priority_bucket.clone(),
            ledger.clone(),
            stats.clone(),
            active_elections.clone(),
        ));

        Self {
            priority,
            optimistic,
            hinted,
            manual,
            notify_listener: OutputListenerMt::new(),
        }
    }

    pub fn activate_successors(&self, tx: &LmdbReadTransaction, block: &SavedBlock) {
        self.priority.activate_successors(tx, block);
    }

    pub fn activate_backlog(
        &self,
        txn: &dyn Transaction,
        account: &Account,
        account_info: &AccountInfo,
        conf_info: &ConfirmationHeightInfo,
    ) {
        self.optimistic.activate(account, account_info, conf_info);
        self.priority
            .activate_with_info(txn, account, account_info, conf_info);
    }

    pub fn activate(&self, tx: &dyn Transaction, account: &Account) -> bool {
        self.priority.activate(tx, account)
    }

    pub fn notify(&self) {
        self.notify_listener.emit(());
        self.priority.notify();
        self.hinted.notify();
        self.optimistic.notify();
    }

    pub fn add_manual(&self, block: SavedBlock) {
        self.manual.push(block, None);
    }

    pub fn start(&self, priority_scheduler_enabled: bool) {
        self.hinted.start();
        self.manual.start();
        self.optimistic.start();
        if priority_scheduler_enabled {
            self.priority.start();
        }
    }

    pub fn track_notify(&self) -> Arc<OutputTrackerMt<()>> {
        self.notify_listener.track()
    }

    pub fn stop(&self) {
        self.hinted.stop();
        self.manual.stop();
        self.optimistic.stop();
        self.priority.stop();
    }

    pub fn container_info(&self) -> ContainerInfo {
        ContainerInfo::builder()
            .node("hinted", self.hinted.container_info())
            .node("manual", self.manual.container_info())
            .node("optimistic", self.optimistic.container_info())
            .node("priority", self.priority.container_info())
            .finish()
    }
}
