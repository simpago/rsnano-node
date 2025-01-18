use crate::cli::get_path;
use anyhow::{anyhow, Result};
use clap::{ArgGroup, Parser};
use rsnano_node::{config::NetworkConstants, NodeBuilder};
use std::sync::{Arc, Condvar, Mutex};

#[derive(Parser)]
#[command(group = ArgGroup::new("input")
    .args(&["data_path", "network"]))]
pub(crate) struct InitializeArgs {
    /// Uses the supplied path as the data directory
    #[arg(long, group = "input")]
    data_path: Option<String>,
    /// Uses the supplied network (live, test, beta or dev)
    #[arg(long, group = "input")]
    network: Option<String>,
}

impl InitializeArgs {
    pub(crate) async fn initialize(&self) -> Result<()> {
        let path = get_path(&self.data_path, &self.network);

        std::fs::create_dir_all(&path).map_err(|e| anyhow!("Create dir failed: {:?}", e))?;

        let node = NodeBuilder::new(NetworkConstants::active_network())
            .data_path(path)
            .finish()
            .unwrap();

        let node = Arc::new(node);
        node.start();

        let finished = Arc::new((Mutex::new(false), Condvar::new()));
        let finished_clone = finished.clone();

        node.stop();
        *finished_clone.0.lock().unwrap() = true;
        finished_clone.1.notify_all();

        let guard = finished.0.lock().unwrap();
        drop(finished.1.wait_while(guard, |g| !*g).unwrap());

        Ok(())
    }
}
