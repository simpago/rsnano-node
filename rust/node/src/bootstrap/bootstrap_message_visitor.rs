use std::sync::{Arc, Weak};

use rsnano_core::work::WorkThresholds;
use rsnano_ledger::Ledger;
use rsnano_messages::Message;

use crate::{
    block_processing::BlockProcessor,
    config::NodeFlags,
    stats::Stats,
    transport::ResponseServerImpl,
    utils::{AsyncRuntime, ThreadPool},
};

use super::{
    BootstrapInitiator, BulkPullAccountServer, BulkPullServer, BulkPushServer, FrontierReqServer,
};

pub struct BootstrapMessageVisitorImpl {
    pub async_rt: Arc<AsyncRuntime>,
    pub ledger: Arc<Ledger>,
    pub connection: Arc<ResponseServerImpl>,
    pub thread_pool: Weak<dyn ThreadPool>,
    pub block_processor: Weak<BlockProcessor>,
    pub bootstrap_initiator: Weak<BootstrapInitiator>,
    pub stats: Arc<Stats>,
    pub work_thresholds: WorkThresholds,
    pub flags: NodeFlags,
}

impl BootstrapMessageVisitorImpl {
    pub fn received(&mut self, message: &Message) -> bool {
        match message {
            Message::BulkPull(payload) => {
                if self.flags.disable_bootstrap_bulk_pull_server {
                    return false;
                }

                let Some(thread_pool) = self.thread_pool.upgrade() else {
                    return false;
                };

                let payload = payload.clone();
                let connection = Arc::clone(&self.connection);
                let ledger = Arc::clone(&self.ledger);
                let thread_pool2 = Arc::clone(&thread_pool);
                let runtime = self.async_rt.clone();
                thread_pool.push_task(Box::new(move || {
                    // TODO from original code: Add completion callback to bulk pull server
                    // TODO from original code: There should be no need to re-copy message as unique pointer, refactor those bulk/frontier pull/push servers
                    let mut bulk_pull_server =
                        BulkPullServer::new(payload, connection, ledger, thread_pool2, runtime);
                    bulk_pull_server.send_next();
                }));

                true
            }
            Message::BulkPullAccount(payload) => {
                if self.flags.disable_bootstrap_bulk_pull_server {
                    return false;
                }
                let Some(thread_pool) = self.thread_pool.upgrade() else {
                    return false;
                };

                let payload = payload.clone();
                let connection = Arc::clone(&self.connection);
                let ledger = Arc::clone(&self.ledger);
                let thread_pool2 = Arc::clone(&thread_pool);
                let runtime = self.async_rt.clone();
                thread_pool.push_task(Box::new(move || {
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

                true
            }
            Message::BulkPush => {
                let Some(thread_pool) = self.thread_pool.upgrade() else {
                    return false;
                };
                let Some(block_processor) = self.block_processor.upgrade() else {
                    return false;
                };
                let Some(bootstrap_initiator) = self.bootstrap_initiator.upgrade() else {
                    return false;
                };
                let connection = Arc::clone(&self.connection);
                let thread_pool2 = Arc::clone(&thread_pool);
                let stats = Arc::clone(&self.stats);
                let work_thresholds = self.work_thresholds.clone();
                let async_rt = Arc::clone(&self.async_rt);
                thread_pool.push_task(Box::new(move || {
                    // original code TODO: Add completion callback to bulk pull server
                    let bulk_push_server = BulkPushServer::new(
                        async_rt,
                        connection,
                        thread_pool2,
                        block_processor,
                        bootstrap_initiator,
                        stats,
                        work_thresholds,
                    );
                    bulk_push_server.throttled_receive();
                }));

                true
            }
            Message::FrontierReq(payload) => {
                let Some(thread_pool) = self.thread_pool.upgrade() else {
                    return false;
                };

                let request = payload.clone();
                let connection = Arc::clone(&self.connection);
                let ledger = Arc::clone(&self.ledger);
                let thread_pool2 = Arc::clone(&thread_pool);
                let runtime = self.async_rt.clone();
                thread_pool.push_task(Box::new(move || {
                    // original code TODO: There should be no need to re-copy message as unique pointer, refactor those bulk/frontier pull/push servers
                    let response =
                        FrontierReqServer::new(connection, request, thread_pool2, ledger, runtime);
                    response.send_next();
                }));

                true
            }
            _ => false,
        }
    }
}
