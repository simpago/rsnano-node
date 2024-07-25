use crate::{
    block_processing::BlockProcessor,
    config::{NetworkConstants, NodeFlags},
    stats::Stats,
    transport::{RealtimeMessageVisitor, RealtimeMessageVisitorImpl, ResponseServerImpl},
    utils::{AsyncRuntime, ThreadPool, ThreadPoolImpl},
};
use rsnano_ledger::Ledger;
use std::sync::{Arc, Weak};

use super::{BootstrapInitiator, BootstrapMessageVisitorImpl};

pub struct BootstrapMessageVisitorFactory {
    async_rt: Arc<AsyncRuntime>,
    stats: Arc<Stats>,
    network_constants: NetworkConstants,
    ledger: Arc<Ledger>,
    thread_pool: Weak<dyn ThreadPool>,
    block_processor: Weak<BlockProcessor>,
    bootstrap_initiator: Weak<BootstrapInitiator>,
    flags: NodeFlags,
}

impl BootstrapMessageVisitorFactory {
    pub fn new(
        async_rt: Arc<AsyncRuntime>,
        stats: Arc<Stats>,
        network_constants: NetworkConstants,
        ledger: Arc<Ledger>,
        thread_pool: Arc<dyn ThreadPool>,
        block_processor: Arc<BlockProcessor>,
        bootstrap_initiator: Arc<BootstrapInitiator>,
        flags: NodeFlags,
    ) -> Self {
        Self {
            async_rt,
            stats,
            network_constants,
            ledger,
            thread_pool: Arc::downgrade(&thread_pool),
            block_processor: Arc::downgrade(&block_processor),
            bootstrap_initiator: Arc::downgrade(&bootstrap_initiator),
            flags,
        }
    }

    pub fn new_null() -> Self {
        let thread_pool: Arc<dyn ThreadPool> = Arc::new(ThreadPoolImpl::new_test_instance());
        let ledger = Arc::new(Ledger::new_null());
        Self {
            async_rt: Arc::new(AsyncRuntime::default()),
            stats: Arc::new(Stats::default()),
            network_constants: NetworkConstants::empty(),
            ledger: ledger.clone(),
            thread_pool: Arc::downgrade(&thread_pool),
            block_processor: Arc::downgrade(&Arc::new(BlockProcessor::new_test_instance(ledger))),
            bootstrap_initiator: Arc::downgrade(&Arc::new(BootstrapInitiator::new_null())),
            flags: NodeFlags::default(),
        }
    }

    pub fn realtime_visitor(
        &self,
        server: Arc<ResponseServerImpl>,
    ) -> Box<dyn RealtimeMessageVisitor> {
        Box::new(RealtimeMessageVisitorImpl::new(
            server,
            Arc::clone(&self.stats),
        ))
    }

    pub fn bootstrap_visitor(
        &self,
        server: Arc<ResponseServerImpl>,
    ) -> BootstrapMessageVisitorImpl {
        BootstrapMessageVisitorImpl {
            async_rt: Arc::clone(&self.async_rt),
            ledger: Arc::clone(&self.ledger),
            connection: server,
            thread_pool: self.thread_pool.clone(),
            block_processor: self.block_processor.clone(),
            bootstrap_initiator: self.bootstrap_initiator.clone(),
            stats: Arc::clone(&self.stats),
            work_thresholds: self.network_constants.work.clone(),
            flags: self.flags.clone(),
        }
    }
}
