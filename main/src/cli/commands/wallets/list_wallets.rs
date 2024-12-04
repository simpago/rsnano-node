use crate::cli::get_path;
use anyhow::{anyhow, Result};
use clap::{ArgGroup, Parser};
use rsnano_core::Account;
use rsnano_node::wallets::Wallets;
use rsnano_store_lmdb::LmdbEnv;
use std::sync::Arc;

#[derive(Parser)]
#[command(group = ArgGroup::new("input")
    .args(&["data_path", "network"]))]
pub(crate) struct ListWalletsArgs {
    /// Uses the supplied path as the data directory
    #[arg(long, group = "input")]
    data_path: Option<String>,
    /// Uses the supplied network (live, test, beta or dev)
    #[arg(long, group = "input")]
    network: Option<String>,
}

impl ListWalletsArgs {
    pub(crate) async fn list_wallets(&self) -> Result<()> {
        let path = get_path(&self.data_path, &self.network).join("wallets.ldb");
        let env = Arc::new(LmdbEnv::new(&path)?);
        let wallets = Arc::new(Wallets::new_null_with_env(
            env.clone(),
            tokio::runtime::Handle::current(),
        ));

        let mut txn = env.tx_begin_read();

        let wallet_ids = wallets.get_wallet_ids(&mut txn);

        for wallet_id in wallet_ids {
            println!("{:?}", wallet_id);
            let accounts = wallets
                .get_accounts_of_wallet(&wallet_id)
                .map_err(|e| anyhow!("Failed to get accounts of wallets: {:?}", e))?;
            if !accounts.is_empty() {
                for account in accounts {
                    println!("{:?}", Account::encode_account(&account));
                }
            }
        }

        Ok(())
    }
}
