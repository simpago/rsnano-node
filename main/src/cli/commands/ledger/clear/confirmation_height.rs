use crate::cli::get_path;
use anyhow::Result;
use clap::{ArgGroup, Parser};
use rsnano_core::{Account, ConfirmationHeightInfo, Networks};
use rsnano_ledger::LedgerConstants;
use rsnano_node::config::NetworkConstants;
use rsnano_store_lmdb::{LmdbConfirmationHeightStore, LmdbEnv};
use std::sync::Arc;

#[derive(Parser)]
#[command(group = ArgGroup::new("input1")
    .args(&["account", "all"])
    .required(true))]
#[command(group = ArgGroup::new("input2")
    .args(&["data_path", "network"]))]
pub(crate) struct ConfirmationHeightArgs {
    /// Clears the confirmation height of the account
    #[arg(long, group = "input1")]
    account: Option<String>,
    /// Clears the confirmation height of all accounts
    #[arg(long, group = "input1")]
    all: bool,
    /// Uses the supplied path as the data directory
    #[arg(long, group = "input")]
    data_path: Option<String>,
    /// Uses the supplied network (live, test, beta or dev)
    #[arg(long, group = "input")]
    network: Option<String>,
}

impl ConfirmationHeightArgs {
    pub(crate) fn confirmation_height(&self) -> Result<()> {
        let path = get_path(&self.data_path, &self.network).join("data.ldb");

        let genesis_block = match NetworkConstants::active_network() {
            Networks::NanoDevNetwork => LedgerConstants::dev().genesis_block,
            Networks::NanoBetaNetwork => LedgerConstants::beta().genesis_block,
            Networks::NanoLiveNetwork => LedgerConstants::live().genesis_block,
            Networks::NanoTestNetwork => LedgerConstants::test().genesis_block,
            Networks::Invalid => unreachable!(),
        };

        let genesis_account = genesis_block.account();
        let genesis_hash = genesis_block.hash();

        let env = Arc::new(LmdbEnv::new(&path)?);

        let confirmation_height_store = LmdbConfirmationHeightStore::new(env.clone())?;

        let mut txn = env.tx_begin_write();

        if let Some(account_hex) = &self.account {
            let account = Account::decode_account(account_hex)?;
            let mut conf_height_reset_num = 0;
            let mut info = confirmation_height_store
                .get(&txn, &account)
                .expect("Could not find account");
            if account == genesis_account {
                conf_height_reset_num += 1;
                info.height = conf_height_reset_num;
                info.frontier = genesis_hash;
                confirmation_height_store.put(&mut txn, &account, &info);
            } else {
                confirmation_height_store.del(&mut txn, &account);
            }
            println!(
                "Confirmation height of account {:?} is set to {:?}",
                account_hex, conf_height_reset_num
            );
        } else {
            confirmation_height_store.clear(&mut txn);
            confirmation_height_store.put(
                &mut txn,
                &genesis_account,
                &ConfirmationHeightInfo::new(1, genesis_hash),
            );
            println!("Confirmation heights of all accounts (except genesis which is set to 1) are set to 0");
        }

        Ok(())
    }
}
