use super::{
    BootstrapAttemptWallet, BootstrapClient, BootstrapConnections, BootstrapConnectionsExt,
    BootstrapInitiator, BootstrapInitiatorExt,
};
use crate::{
    bootstrap::BootstrapAttemptTrait,
    stats::{DetailType, Direction, StatType, Stats},
    transport::TrafficType,
    utils::{AsyncRuntime, ThreadPool},
};
use async_trait::async_trait;
use rsnano_core::{
    utils::{BufferReader, Deserialize, FixedSizeSerialize},
    Account, Amount, BlockHash,
};
use rsnano_ledger::Ledger;
use rsnano_messages::{BulkPullAccount, BulkPullAccountFlags, Message};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tracing::{debug, trace};

pub struct BulkPullAccountClient {
    connection: Arc<BootstrapClient>,
    attempt: Arc<BootstrapAttemptWallet>,
    account: Account,
    receive_minimum: Amount,
    stats: Arc<Stats>,
    pull_blocks: AtomicU64,
    connections: Arc<BootstrapConnections>,
    ledger: Arc<Ledger>,
    bootstrap_initiator: Arc<BootstrapInitiator>,
    runtime: Arc<AsyncRuntime>,
    workers: Arc<dyn ThreadPool>,
}

impl BulkPullAccountClient {
    pub fn new(
        connection: Arc<BootstrapClient>,
        attempt: Arc<BootstrapAttemptWallet>,
        account: Account,
        receive_minimum: Amount,
        stats: Arc<Stats>,
        connections: Arc<BootstrapConnections>,
        ledger: Arc<Ledger>,
        bootstrap_initiator: Arc<BootstrapInitiator>,
        runtime: Arc<AsyncRuntime>,
        workers: Arc<dyn ThreadPool>,
    ) -> Self {
        attempt.notify();
        Self {
            connection,
            attempt,
            account,
            receive_minimum,
            stats,
            pull_blocks: AtomicU64::new(0),
            connections,
            ledger,
            bootstrap_initiator,
            runtime,
            workers,
        }
    }
}

impl Drop for BulkPullAccountClient {
    fn drop(&mut self) {
        self.attempt.pull_finished();
    }
}

#[async_trait]
pub trait BulkPullAccountClientExt {
    fn request(&self);
    async fn receive_pending(&self);
}

#[async_trait]
impl BulkPullAccountClientExt for Arc<BulkPullAccountClient> {
    fn request(&self) {
        let req = Message::BulkPullAccount(BulkPullAccount {
            account: self.account,
            minimum_amount: self.receive_minimum,
            flags: BulkPullAccountFlags::PendingHashAndAmount,
        });

        trace!(
            account = self.account.encode_account(),
            connection = self.connection.channel_string(),
            "requesting pending"
        );

        if self.attempt.should_log() {
            debug!("Accounts in pull queue: {}", self.attempt.wallet_size());
        }

        let self_l = Arc::clone(self);
        self.runtime.tokio.spawn(async move {
            match self_l
                .connection
                .get_channel()
                .send(&req, TrafficType::Generic)
                .await
            {
                Ok(()) => {
                    self_l.receive_pending().await;
                }
                Err(e) => {
                    debug!(
                        "Error starting bulk pull request to: {} ({:?})",
                        self_l.connection.channel_string(),
                        e
                    );
                    self_l.stats.inc_dir(
                        StatType::Bootstrap,
                        DetailType::BulkPullErrorStartingRequest,
                        Direction::In,
                    );

                    self_l.attempt.requeue_pending(self_l.account);
                }
            }
        });
    }

    async fn receive_pending(&self) {
        let mut buffer = [0; 256];
        if let Err(e) = self
            .connection
            .get_channel()
            .read(
                &mut buffer,
                BlockHash::serialized_size() + Amount::serialized_size(),
            )
            .await
        {
            debug!("Error while receiving bulk pull account frontier: {:?}", e);
            self.attempt.requeue_pending(self.account);
            return;
        }

        let mut reader = BufferReader::new(&buffer);
        let pending = BlockHash::deserialize(&mut reader).unwrap();
        let balance = Amount::deserialize(&mut reader).unwrap();
        if self.pull_blocks.load(Ordering::SeqCst) == 0 || !pending.is_zero() {
            if self.pull_blocks.load(Ordering::SeqCst) == 0 || balance >= self.receive_minimum {
                let this_l = Arc::clone(self);
                self.workers.push_task(Box::new(move || {
                    this_l.pull_blocks.fetch_add(1, Ordering::SeqCst);
                    {
                        if !pending.is_zero() {
                            if !this_l
                                .ledger
                                .any()
                                .block_exists_or_pruned(&this_l.ledger.read_txn(), &pending)
                            {
                                this_l.bootstrap_initiator.bootstrap_lazy(
                                    pending.into(),
                                    false,
                                    "".to_string(),
                                );
                            }
                        }
                    }
                    let runtime = this_l.runtime.clone();
                    runtime
                        .tokio
                        .spawn(async move { this_l.receive_pending().await });
                }));
            } else {
                self.attempt.requeue_pending(self.account);
            }
        } else {
            self.connections
                .pool_connection(Arc::clone(&self.connection), false, false);
        }
    }
}
