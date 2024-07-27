use crate::cli::{commands::node::read_node_config_toml, get_path};
use anyhow::Result;
use clap::{ArgGroup, Parser};
use rsnano_core::utils::get_cpu_count;
use rsnano_node::{
    config::{get_node_toml_config_path, DaemonConfig, DaemonConfigToml, NetworkConstants},
    NetworkParams,
};
use toml::from_str;

#[derive(Parser)]
#[command(group = ArgGroup::new("input1")
    .args(&["node", "rpc"])
    .required(true))]
#[command(group = ArgGroup::new("input2")
    .args(&["data_path", "network"]))]
pub(crate) struct UpdateConfigArgs {
    /// Prints the current node config
    #[arg(long, group = "input1")]
    node: bool,
    /// Prints the current rpc config
    #[arg(long, group = "input1")]
    rpc: bool,
    /// Uses the supplied path as the data directory
    #[arg(long, group = "input2")]
    data_path: Option<String>,
    /// Uses the supplied network (live, test, beta or dev)
    #[arg(long, group = "input2")]
    network: Option<String>,
}

impl UpdateConfigArgs {
    pub(crate) fn update_config(&self) -> Result<()> {
        let path = get_path(&self.data_path, &self.network);

        let network_params = NetworkParams::new(NetworkConstants::active_network());

        if self.node {
            let node_toml_config_path = get_node_toml_config_path(&path);

            if node_toml_config_path.exists() {
                let toml_str = read_node_config_toml(&node_toml_config_path)?;
                let current_toml_daemon_config: DaemonConfigToml = from_str(&toml_str)?;
                let default_toml_daemon_config: DaemonConfigToml =
                    DaemonConfig::new(&network_params, get_cpu_count())?.into();
                let merged_config =
                    current_toml_daemon_config.merge_defaults(&default_toml_daemon_config)?;

                println!("{}", merged_config);
            }
        } else {
            // todo: rpc config
        }

        Ok(())
    }
}
