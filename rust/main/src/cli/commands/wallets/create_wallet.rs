use crate::cli::get_path;
use anyhow::{anyhow, Result};
use clap::{ArgGroup, Parser};
use rand::{thread_rng, Rng};
use rsnano_core::{RawKey, WalletId};
use rsnano_node::wallets::{Wallets, WalletsExt};
use rsnano_store_lmdb::LmdbEnv;
use std::sync::Arc;

#[derive(Parser)]
#[command(group = ArgGroup::new("input")
    .args(&["data_path", "network"]))]
pub(crate) struct CreateWalletArgs {
    /// Optional seed of the new wallet
    #[arg(long)]
    seed: Option<String>,
    /// Optional password of the new wallet
    #[arg(long)]
    password: Option<String>,
    /// Uses the supplied path as the data directory
    #[arg(long, group = "input")]
    data_path: Option<String>,
    /// Uses the supplied network (live, test, beta or dev)
    #[arg(long, group = "input")]
    network: Option<String>,
}

impl CreateWalletArgs {
    pub(crate) fn create_wallet(&self) -> Result<()> {
        let path = get_path(&self.data_path, &self.network).join("wallets.ldb");

        let wallet_id = WalletId::from_bytes(thread_rng().gen());

        let env = Arc::new(LmdbEnv::new(&path)?);

        let wallets = Arc::new(Wallets::new_null_with_env(env)?);

        wallets.create(wallet_id);

        println!("{:?}", wallet_id);

        let password = self.password.clone().unwrap_or_default();

        wallets
            .rekey(&wallet_id, &password)
            .map_err(|e| anyhow!("Failed to set wallet password: {:?}", e))?;

        wallets.ensure_wallet_is_unlocked(wallet_id, &password);

        if let Some(seed) = &self.seed {
            let key = RawKey::decode_hex(seed)?;

            wallets
                .change_seed(wallet_id, &key, 0)
                .map_err(|e| anyhow!("Failed to set wallet seed: {:?}", e))?;
        }

        Ok(())
    }
}
