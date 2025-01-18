use super::{Wallets, WalletsExt};
use crate::utils::ThreadPool;
use std::{sync::Arc, time::Duration};

pub(crate) struct ReceivableSearch {
    pub wallets: Arc<Wallets>,
    pub workers: Arc<dyn ThreadPool>,
    pub interval: Duration,
}

impl ReceivableSearch {
    pub fn start(&self) {
        search_receivables(
            self.wallets.clone(),
            self.workers.clone(),
            self.interval.clone(),
        );
    }
}

fn search_receivables(wallets: Arc<Wallets>, workers: Arc<dyn ThreadPool>, interval: Duration) {
    // Reload wallets from disk
    wallets.reload();
    // Search pending
    wallets.search_receivable_all();

    let wallets_w = Arc::downgrade(&wallets);
    let workers_w = Arc::downgrade(&workers);

    workers.post_delayed(
        interval,
        Box::new(move || {
            let Some(wallets) = wallets_w.upgrade() else {
                return;
            };
            let Some(workers) = workers_w.upgrade() else {
                return;
            };
            search_receivables(wallets, workers, interval);
        }),
    )
}
