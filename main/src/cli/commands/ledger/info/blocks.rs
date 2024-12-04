use crate::cli::get_path;
use anyhow::Result;
use clap::{ArgGroup, Parser};
use rsnano_store_lmdb::LmdbStore;

#[derive(Parser)]
#[command(group = ArgGroup::new("input")
    .args(&["data_path", "network"]))]
pub(crate) struct Blocks {
    /// Uses the supplied path as the data directory
    #[arg(long, group = "input")]
    data_path: Option<String>,
    /// Uses the supplied network (live, test, beta or dev)
    #[arg(long, group = "input")]
    network: Option<String>,
}

impl Blocks {
    pub(crate) fn blocks(&self) -> Result<()> {
        let path = get_path(&self.data_path, &self.network).join("data.ldb");
        let store = LmdbStore::open(&path).build()?;
        let mut txn = store.tx_begin_read();
        let mut iter = store.block.begin(&mut txn);
        let end = store.block.end();

        while iter != end {
            match iter.current() {
                Some((hash, block)) => {
                    println!("{}", hash.to_string());
                    println!("{} \n", block.to_json().unwrap());
                }
                None => break,
            }
            iter.next();
        }

        Ok(())
    }
}
