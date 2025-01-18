use super::Wallets;
use crate::utils::ThreadPool;
use std::{path::PathBuf, sync::Arc, time::Duration};
use tracing::error;

pub(crate) struct WalletBackup {
    pub data_path: PathBuf,
    pub backup_interval: Duration,
    pub workers: Arc<dyn ThreadPool>,
    pub wallets: Arc<Wallets>,
}

impl WalletBackup {
    pub fn start(&self) {
        let mut backup_path = self.data_path.clone();
        backup_path.push("backup");
        ongoing_backup(
            backup_path,
            self.backup_interval.clone(),
            self.workers.clone(),
            self.wallets.clone(),
        );
    }
}

fn ongoing_backup(
    backup_path: PathBuf,
    backup_interval: Duration,
    workers: Arc<dyn ThreadPool>,
    wallets: Arc<Wallets>,
) {
    if let Err(e) = wallets.backup(&backup_path) {
        error!(error = ?e, "Could not create backup of wallets");
    }

    let workers_w = Arc::downgrade(&workers);
    let wallets_w = Arc::downgrade(&wallets);

    workers.post_delayed(
        backup_interval,
        Box::new(move || {
            let Some(workers) = workers_w.upgrade() else {
                return;
            };
            let Some(wallets) = wallets_w.upgrade() else {
                return;
            };
            ongoing_backup(backup_path, backup_interval, workers, wallets);
        }),
    )
}
