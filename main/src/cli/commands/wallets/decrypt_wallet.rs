use crate::cli::get_path;
use anyhow::{anyhow, Result};
use clap::Parser;
use rsnano_core::WalletId;
use rsnano_node::wallets::{Wallets, WalletsExt};
use rsnano_store_lmdb::LmdbEnv;
use std::sync::Arc;

#[derive(Parser)]
#[command(group = clap::ArgGroup::new("input")
    .args(&["data_path", "network"]))]
pub(crate) struct DecryptWalletArgs {
    /// The wallet to be decrypted
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

impl DecryptWalletArgs {
    pub(crate) async fn decrypt_wallet(&self) -> Result<()> {
        let path = get_path(&self.data_path, &self.network).join("wallets.ldb");
        let wallet_id = WalletId::decode_hex(&self.wallet)?;
        let env = Arc::new(LmdbEnv::new(&path)?);
        let wallets = Arc::new(Wallets::new_null_with_env(
            env,
            tokio::runtime::Handle::current(),
        ));

        let password = self.password.clone().unwrap_or_default();

        wallets.ensure_wallet_is_unlocked(wallet_id, &password);

        let seed = wallets
            .get_seed(wallet_id)
            .map_err(|e| anyhow!("Failed to get wallet seed: {:?}", e))?;

        println!("Seed: {:?}", seed);

        Ok(())
    }
}
