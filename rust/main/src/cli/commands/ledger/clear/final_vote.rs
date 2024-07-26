use crate::cli::get_path;
use anyhow::Result;
use clap::{ArgGroup, Parser};
use rsnano_core::Root;
use rsnano_store_lmdb::{LmdbEnv, LmdbFinalVoteStore};
use std::sync::Arc;

#[derive(Parser)]
#[command(group = ArgGroup::new("input1")
    .args(&["root", "all"])
    .required(true))]
#[command(group = ArgGroup::new("input2")
    .args(&["data_path", "network"]))]
pub(crate) struct FinalVoteArgs {
    /// Clears the supplied final vote
    #[arg(long, group = "input1")]
    root: Option<String>,
    /// Clears all final votes (not recommended)
    #[arg(long, group = "input1")]
    all: bool,
    /// Uses the supplied path as the data directory
    #[arg(long, group = "input")]
    data_path: Option<String>,
    /// Uses the supplied network (live, test, beta or dev)
    #[arg(long, group = "input")]
    network: Option<String>,
}

impl FinalVoteArgs {
    pub(crate) fn final_vote(&self) -> Result<()> {
        let path = get_path(&self.data_path, &self.network).join("data.ldb");

        let env = Arc::new(LmdbEnv::new(&path)?);

        let final_vote_store = LmdbFinalVoteStore::new(env.clone())?;

        let mut txn = env.tx_begin_write();

        if let Some(root) = &self.root {
            let root_decoded = Root::decode_hex(root)?;
            final_vote_store.del(&mut txn, &root_decoded);
            println!("Successfully cleared final vote");
        } else {
            final_vote_store.clear(&mut txn);
            println!("All final votes were cleared from the database");
        }

        Ok(())
    }
}
