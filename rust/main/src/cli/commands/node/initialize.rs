use crate::cli::get_path;
use anyhow::{anyhow, Result};
use clap::{ArgGroup, Parser};
use rsnano_core::{utils::get_cpu_count, work::WorkPoolImpl};
use rsnano_node::{
    config::{NetworkConstants, NodeConfig, NodeFlags},
    node::{Node, NodeExt},
    transport::NullSocketObserver,
    utils::AsyncRuntime,
    NetworkParams,
};
use std::{
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};

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
    pub(crate) fn initialize(&self) -> Result<()> {
        let path = get_path(&self.data_path, &self.network);

        let network_params = NetworkParams::new(NetworkConstants::active_network());

        std::fs::create_dir_all(&path).map_err(|e| anyhow!("Create dir failed: {:?}", e))?;

        let config = NodeConfig::new(
            Some(network_params.network.default_node_port),
            &network_params,
            get_cpu_count(),
        );

        let flags = NodeFlags::new();
        let async_rt = Arc::new(AsyncRuntime::default());

        let work = Arc::new(WorkPoolImpl::new(
            network_params.work.clone(),
            config.work_threads as usize,
            Duration::from_nanos(config.pow_sleep_interval_ns as u64),
        ));

        let node = Arc::new(Node::new(
            async_rt,
            path,
            config,
            network_params,
            flags,
            work,
            Arc::new(NullSocketObserver::new()),
            Box::new(|_, _, _, _, _, _| {}),
            Box::new(|_, _| {}),
            Box::new(|_, _, _, _| {}),
        ));

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
