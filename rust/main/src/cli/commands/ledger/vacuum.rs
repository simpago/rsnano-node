use crate::cli::get_path;
use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use rsnano_store_lmdb::LmdbStore;
use std::fs;

#[derive(Parser)]
pub(crate) struct VacuumArgs {
    /// Uses the supplied path as the data directory
    #[arg(long, group = "input")]
    data_path: Option<String>,
    /// Uses the supplied network (live, test, beta or dev)
    #[arg(long, group = "input")]
    network: Option<String>,
}

impl VacuumArgs {
    pub(crate) fn vacuum(&self) -> Result<()> {
        let data_path = get_path(&self.data_path, &self.network);
        let source_path = data_path.join("data.ldb");
        let backup_path = data_path.join("backup.vacuum.ldb");
        let vacuum_path = data_path.join("vacuumed.ldb");

        println!("Vacuuming database copy in {:?}", data_path);
        println!("This may take a while...");

        let store = LmdbStore::open(&source_path).build()?;

        store.copy_db(&vacuum_path)?;

        println!("Finalizing");

        fs::rename(&source_path, &backup_path).context("Failed to rename source to backup")?;
        fs::rename(&vacuum_path, &source_path).context("Failed to rename vacuum to source")?;
        fs::remove_file(&backup_path).context("Failed to remove backup file")?;

        println!("Vacuum completed");

        Ok(())
    }
}
