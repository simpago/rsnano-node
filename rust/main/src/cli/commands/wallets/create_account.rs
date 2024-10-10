use crate::cli::get_path;
use anyhow::{anyhow, Result};
use clap::{ArgGroup, Parser};
use rsnano_core::{Account, WalletId};
use rsnano_node::wallets::{Wallets, WalletsExt};
use rsnano_store_lmdb::LmdbEnv;
use std::sync::Arc;

#[derive(Parser)]
#[command(group = ArgGroup::new("input")
    .args(&["data_path", "network"]))]
pub(crate) struct CreateAccountArgs {
    /// Creates an account in the supplied <wallet>
    #[arg(long)]
    wallet: String,
    /// Optional password to unlock the wallet
    #[arg(long)]
    password: Option<String>,
    /// Uses the supplied path as the data directory
    #[arg(long, group = "input")]
    data_path: Option<String>,
    /// Uses the supplied network (live, test, beta or dev)
    #[arg(long, group = "input")]
    network: Option<String>,
}

impl CreateAccountArgs {
    pub(crate) async fn create_account(&self) -> Result<()> {
        let path = get_path(&self.data_path, &self.network).join("wallets.ldb");
        let env = Arc::new(LmdbEnv::new(&path)?);

        let wallets = Arc::new(Wallets::new_null_with_env(
            env,
            tokio::runtime::Handle::current(),
        ));

        let wallet = WalletId::decode_hex(&self.wallet)?;
        let password = self.password.clone().unwrap_or_default();

        wallets.ensure_wallet_is_unlocked(wallet, &password);

        let public_key = wallets
            .deterministic_insert2(&wallet, false)
            .map_err(|e| anyhow!("Failed to insert wallet: {:?}", e))?;

        println!("Account: {:?}", Account::from(public_key).encode_account());

        Ok(())
    }
}
