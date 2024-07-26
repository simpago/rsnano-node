use crate::cli::get_path;
use anyhow::{anyhow, Result};
use clap::{ArgGroup, Parser};
use rsnano_core::WalletId;
use rsnano_node::wallets::{Wallets, WalletsExt};
use rsnano_store_lmdb::LmdbEnv;
use std::sync::Arc;

#[derive(Parser)]
#[command(group = ArgGroup::new("input")
    .args(&["data_path", "network"]))]
pub(crate) struct GetWalletRepresentativeArgs {
    /// Gets the representative of the supplied <wallet>
    #[arg(long)]
    wallet: String,
    /// Optional password to unlock the wallet
    #[arg(long)]
    password: Option<String>,
    #[arg(long)]
    /// Uses the supplied path as the data directory
    #[arg(long, group = "input")]
    data_path: Option<String>,
    /// Uses the supplied network (live, test, beta or dev)
    #[arg(long, group = "input")]
    network: Option<String>,
}

impl GetWalletRepresentativeArgs {
    pub(crate) fn get_wallet_representative(&self) -> Result<()> {
        let wallet_id = WalletId::decode_hex(&self.wallet)?;

        let path = get_path(&self.data_path, &self.network).join("wallets.ldb");

        let env = Arc::new(LmdbEnv::new(&path)?);

        let wallets = Arc::new(Wallets::new_null_with_env(env)?);

        let password = self.password.clone().unwrap_or_default();

        wallets.ensure_wallet_is_unlocked(wallet_id, &password);

        let representative = wallets
            .get_representative(wallet_id)
            .map_err(|e| anyhow!("Failed to get wallet representative: {:?}", e))?;

        println!("Representative: {:?}", &representative.encode_account());

        Ok(())
    }
}
