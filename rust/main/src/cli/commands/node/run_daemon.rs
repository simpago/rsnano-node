use super::read_node_config_toml;
use crate::cli::{get_path, init_tracing};
use anyhow::{anyhow, Result};
use clap::{ArgGroup, Parser};
use rsnano_core::work::WorkPoolImpl;
use rsnano_node::{
    config::{
        get_node_toml_config_path, DaemonConfigToml, NetworkConstants, NodeConfig, NodeFlags,
    },
    node::{Node, NodeExt},
    transport::NullSocketObserver,
    utils::AsyncRuntime,
    NetworkParams,
};
use std::{
    fs::create_dir_all,
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};
use toml::from_str;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(group = ArgGroup::new("input")
    .args(&["data_path", "network"]))]
pub(crate) struct RunDaemonArgs {
    /// Uses the supplied path as the data directory
    #[arg(long, group = "input")]
    data_path: Option<String>,
    /// Uses the supplied network (live, test, beta or dev)
    #[arg(long, group = "input")]
    network: Option<String>,
    /// Passes node configuration values
    /// This takes precedence over any values in the configuration file
    /// This option can be repeated multiple times
    #[arg(long, verbatim_doc_comment)]
    config_overrides: Option<Vec<String>>,
    /// Passes RPC configuration values
    /// This takes precedence over any values in the configuration file
    /// This option can be repeated multiple times.
    #[arg(long, verbatim_doc_comment)]
    rpc_config_overrides: Option<Vec<String>>,
    /// Disables activate_successors in active_elections
    #[arg(long)]
    disable_activate_successors: bool,
    /// Turns off automatic wallet backup process
    #[arg(long)]
    disable_backup: bool,
    /// Turns off use of lazy bootstrap
    #[arg(long)]
    disable_lazy_bootstrap: bool,
    /// Turns off use of legacy bootstrap
    #[arg(long)]
    disable_legacy_bootstrap: bool,
    /// Turns off use of wallet-based bootstrap
    #[arg(long)]
    disable_wallet_bootstrap: bool,
    /// Turns off listener on the bootstrap network so incoming TCP (bootstrap) connections are rejected
    /// This does not impact TCP traffic for the live network
    #[arg(long, verbatim_doc_comment)]
    disable_bootstrap_listener: bool,
    /// Disables the legacy bulk pull server for bootstrap operations
    #[arg(long)]
    disable_bootstrap_bulk_pull_server: bool,
    /// Disables the legacy bulk push client for bootstrap operations
    #[arg(long)]
    disable_bootstrap_bulk_push_client: bool,
    /// Turns off the ability for ongoing bootstraps to occur
    #[arg(long)]
    disable_ongoing_bootstrap: bool,
    /// Disables ascending bootstrap
    #[arg(long)]
    disable_ascending_bootstrap: bool,
    /// Turns off the request loop
    #[arg(long)]
    disable_request_loop: bool,
    /// Turns off the rep crawler process
    #[arg(long)]
    disable_rep_crawler: bool,
    /// Turns off use of TCP live network (TCP for bootstrap will remain available)
    #[arg(long)]
    disable_tcp_realtime: bool,
    /// Does not provide any telemetry data to nodes requesting it
    /// Responses are still made to requests, but they will have an empty payload
    #[arg(long, verbatim_doc_comment)]
    disable_providing_telemetry_metrics: bool,
    /// Disables ongoing telemetry requests to peers
    #[arg(long)]
    disable_ongoing_telemetry_requests: bool,
    /// Disables deletion of unchecked blocks after processing
    #[arg(long)]
    disable_block_processor_unchecked_deletion: bool,
    /// Disables block republishing by disabling the local_block_broadcaster component
    #[arg(long)]
    disable_block_processor_republishing: bool,
    /// Allows multiple connections to the same peer in bootstrap attempts
    #[arg(long)]
    allow_bootstrap_peers_duplicates: bool,
    /// Enables experimental ledger pruning
    #[arg(long)]
    enable_pruning: bool,
    /// Increases bootstrap processor limits to allow more blocks before hitting full state and verify/write more per database call
    /// Also disables deletion of processed unchecked blocks
    #[arg(long, verbatim_doc_comment)]
    fast_bootstrap: bool,
    /// Increases block processor transaction batch write size
    #[arg(long, verbatim_doc_comment)]
    block_processor_batch_size: Option<usize>,
    /// Increases block processor allowed blocks queue size before dropping live network packets and holding bootstrap download
    #[arg(long, verbatim_doc_comment)]
    block_processor_full_size: Option<usize>,
    /// Increases batch signature verification size in block processor
    #[arg(long, verbatim_doc_comment)]
    block_processor_verification_size: Option<usize>,
    /// Increases vote processor queue size before dropping votes
    #[arg(long)]
    vote_processor_capacity: Option<usize>,
}

impl RunDaemonArgs {
    pub(crate) fn run_daemon(&self) -> Result<()> {
        let dirs = std::env::var(EnvFilter::DEFAULT_ENV).unwrap_or(String::from(
            "rsnano_ffi=debug,rsnano_node=debug,rsnano_messages=debug,rsnano_ledger=debug,rsnano_store_lmdb=debug,rsnano_core=debug",
        ));

        init_tracing(dirs);

        let path = get_path(&self.data_path, &self.network);

        let network_params = NetworkParams::new(NetworkConstants::active_network());

        create_dir_all(&path).map_err(|e| anyhow!("Create dir failed: {:?}", e))?;

        let node_toml_config_path = get_node_toml_config_path(&path);

        let mut node_config = NodeConfig::default();

        if node_toml_config_path.exists() {
            let toml_str = read_node_config_toml(&node_toml_config_path)?;
            let toml_daemon_config: DaemonConfigToml = from_str(&toml_str)?;
            if let Some(toml_node_config) = toml_daemon_config.node {
                node_config = toml_node_config.into();
            }
        }

        let mut flags = NodeFlags::new();
        self.set_flags(&mut flags);

        let async_rt = Arc::new(AsyncRuntime::default());

        let work = Arc::new(WorkPoolImpl::new(
            network_params.work.clone(),
            node_config.work_threads as usize,
            Duration::from_nanos(node_config.pow_sleep_interval_ns as u64),
        ));

        let node = Arc::new(Node::new(
            async_rt,
            path,
            node_config,
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

        ctrlc::set_handler(move || {
            node.stop();
            *finished_clone.0.lock().unwrap() = true;
            finished_clone.1.notify_all();
        })
        .expect("Error setting Ctrl-C handler");

        let guard = finished.0.lock().unwrap();
        drop(finished.1.wait_while(guard, |g| !*g).unwrap());

        Ok(())
    }

    pub(crate) fn set_flags(&self, node_flags: &mut NodeFlags) {
        if let Some(config_overrides) = &self.config_overrides {
            node_flags.set_config_overrides(config_overrides.clone());
        }
        if let Some(rpc_config_overrides) = &self.rpc_config_overrides {
            node_flags.set_rpc_config_overrides(rpc_config_overrides.clone());
        }
        if self.disable_activate_successors {
            node_flags.set_disable_activate_successors(true);
        }
        if self.disable_backup {
            node_flags.set_disable_backup(true);
        }
        if self.disable_lazy_bootstrap {
            node_flags.set_disable_lazy_bootstrap(true);
        }
        if self.disable_legacy_bootstrap {
            node_flags.set_disable_legacy_bootstrap(true);
        }
        if self.disable_wallet_bootstrap {
            node_flags.set_disable_wallet_bootstrap(true);
        }
        if self.disable_bootstrap_listener {
            node_flags.set_disable_bootstrap_listener(true);
        }
        if self.disable_bootstrap_bulk_pull_server {
            node_flags.set_disable_bootstrap_bulk_pull_server(true);
        }
        if self.disable_bootstrap_bulk_push_client {
            node_flags.set_disable_bootstrap_bulk_push_client(true);
        }
        if self.disable_ongoing_bootstrap {
            node_flags.set_disable_ongoing_bootstrap(true);
        }
        if self.disable_ascending_bootstrap {
            node_flags.set_disable_ascending_bootstrap(true);
        }
        if self.disable_rep_crawler {
            node_flags.set_disable_rep_crawler(true);
        }
        if self.disable_request_loop {
            node_flags.set_disable_request_loop(true);
        }
        if self.disable_tcp_realtime {
            node_flags.set_disable_tcp_realtime(true);
        }
        if self.disable_providing_telemetry_metrics {
            node_flags.set_disable_providing_telemetry_metrics(true);
        }
        if self.disable_ongoing_telemetry_requests {
            node_flags.set_disable_ongoing_telemetry_requests(true);
        }
        if self.disable_block_processor_unchecked_deletion {
            node_flags.set_disable_block_processor_unchecked_deletion(true);
        }
        if self.disable_block_processor_republishing {
            node_flags.set_disable_block_processor_republishing(true);
        }
        if self.allow_bootstrap_peers_duplicates {
            node_flags.set_allow_bootstrap_peers_duplicates(true);
        }
        if self.enable_pruning {
            node_flags.set_enable_pruning(true);
        }
        if self.fast_bootstrap {
            node_flags.set_fast_bootstrap(true);
        }
        if let Some(block_processor_batch_size) = self.block_processor_batch_size {
            node_flags.set_block_processor_batch_size(block_processor_batch_size);
        }
        if let Some(block_processor_full_size) = self.block_processor_full_size {
            node_flags.set_block_processor_full_size(block_processor_full_size);
        }
        if let Some(block_processor_verification_size) = self.block_processor_verification_size {
            node_flags.set_block_processor_verification_size(block_processor_verification_size);
        }
        if let Some(vote_processor_capacity) = self.vote_processor_capacity {
            node_flags.set_vote_processor_capacity(vote_processor_capacity);
        }
    }
}
