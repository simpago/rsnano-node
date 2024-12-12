use crate::cli::get_path;
use anyhow::Result;
use clap::{ArgGroup, Parser};
use rsnano_core::Amount;
use rsnano_ledger::{Ledger, RepWeightCache};
use rsnano_node::{config::NetworkConstants, NetworkParams, OnlineWeightSampler};
use rsnano_store_lmdb::LmdbStore;
use std::sync::Arc;

#[derive(Parser)]
#[command(group = ArgGroup::new("input")
    .args(&["data_path", "network"]))]
pub(crate) struct TrendedOnlineWeightArgs {
    /// Uses the supplied path as the data directory
    #[arg(long, group = "input")]
    data_path: Option<String>,
    /// Uses the supplied network (live, test, beta or dev)
    #[arg(long, group = "input")]
    network: Option<String>,
}

impl TrendedOnlineWeightArgs {
    pub(crate) fn trended_online_weight(&self) -> Result<()> {
        let path = get_path(&self.data_path, &self.network).join("data.ldb");

        let network_params = NetworkParams::new(NetworkConstants::active_network());

        let ledger = Arc::new(Ledger::new(
            Arc::new(LmdbStore::open(&path).build()?),
            network_params.ledger,
            Amount::zero(),
            Arc::new(RepWeightCache::new()),
        )?);

        let sampler = OnlineWeightSampler::new(
            ledger.clone(),
            network_params.node.max_weight_samples as usize,
        );

        let current = sampler.calculate_trend().number();

        println!("Trended Weight {}", current);

        let tx = ledger.store.tx_begin_read();
        for (timestamp, amount) in ledger.store.online_weight.iter(&tx) {
            println!("Timestamp {} Weight {}", timestamp, amount.number());
        }

        Ok(())
    }
}
