use crate::cli::get_path;
use anyhow::Result;
use clap::Parser;
use rsnano_store_lmdb::LmdbStore;

#[derive(Parser)]
pub(crate) struct SnapshotArgs {
    /// Uses the supplied path as the data directory
    #[arg(long, group = "input")]
    data_path: Option<String>,
    /// Uses the supplied network (live, test, beta or dev)
    #[arg(long, group = "input")]
    network: Option<String>,
}

impl SnapshotArgs {
    pub(crate) fn snapshot(&self) -> Result<()> {
        let source_path = get_path(&self.data_path, &self.network).join("data.ldb");
        let snapshot_path = get_path(&self.data_path, &self.network).join("snapshot.ldb");

        println!(
            "Database snapshot of {:?} to {:?} in progress",
            source_path, snapshot_path
        );

        println!("This may take a while...");

        let store = LmdbStore::open(&source_path).build()?;

        store.copy_db(&snapshot_path)?;

        println!(
            "Snapshot completed, This can be found at {:?}",
            snapshot_path
        );

        Ok(())
    }
}
