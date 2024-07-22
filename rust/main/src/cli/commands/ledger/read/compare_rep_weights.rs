use crate::cli::get_path;
use anyhow::{anyhow, Result};
use clap::{ArgGroup, Parser};
use rsnano_core::Amount;
use rsnano_ledger::{Ledger, LedgerCache, RepWeightCache};
use rsnano_node::{config::NetworkConstants, NetworkParams};
use rsnano_store_lmdb::LmdbStore;
use std::sync::Arc;

#[derive(Parser)]
#[command(group = ArgGroup::new("input")
    .args(&["data_path", "network"]))]
pub(crate) struct CompareRepWeightsArgs {
    #[arg(long, group = "input")]
    data_path: Option<String>,
    #[arg(long, group = "input")]
    network: Option<String>,
}

impl CompareRepWeightsArgs {
    pub(crate) fn compare_rep_weights(&self) -> Result<()> {
        let path = get_path(&self.data_path, &self.network).join("data.ldb");

        let network_params = NetworkParams::new(NetworkConstants::active_network());

        let ledger_cache = Arc::new(LedgerCache::new());

        let ledger = Ledger::new(
            Arc::new(
                LmdbStore::open(&path)
                    .build()
                    .map_err(|e| anyhow!("Failed to open store: {:?}", e))?,
            ),
            network_params.ledger,
            Amount::zero(),
            Arc::new(RepWeightCache::new()),
            ledger_cache,
        )?;

        Ok(())
    }
}
