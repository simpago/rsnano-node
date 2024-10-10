use crate::cli::get_path;
use anyhow::Result;
use clap::{ArgGroup, Parser};
use rsnano_node::wallets::Wallets;
use rsnano_store_lmdb::LmdbEnv;
use std::sync::Arc;

#[derive(Parser)]
#[command(group = ArgGroup::new("input")
    .args(&["data_path", "network"]))]
pub(crate) struct ClearSendIdsArgs {
    /// Uses the supplied path as the data directory
    #[arg(long, group = "input")]
    data_path: Option<String>,
    /// Uses the supplied network (live, test, beta or dev)
    #[arg(long, group = "input")]
    network: Option<String>,
}

impl ClearSendIdsArgs {
    pub(crate) async fn clear_send_ids(&self) -> Result<()> {
        let path = get_path(&self.data_path, &self.network).join("wallets.ldb");
        let env = Arc::new(LmdbEnv::new(&path)?);
        let wallets = Wallets::new_null_with_env(env, tokio::runtime::Handle::current());
        wallets.clear_send_ids();
        println!("{}", "Send IDs deleted");
        Ok(())
    }
}
