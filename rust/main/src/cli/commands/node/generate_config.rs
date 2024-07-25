use anyhow::Result;
use clap::{ArgGroup, Parser};
use rsnano_core::work::WorkThresholds;
use rsnano_node::{
    config::{DaemonConfig, NetworkConstants, RpcConfig},
    utils::TomlConfig,
    NetworkParams,
};

#[derive(Parser)]
#[command(group = ArgGroup::new("input1")
    .args(&["node", "rpc"])
    .required(true))]
#[command(group = ArgGroup::new("input2")
    .args(&["data_path", "network"]))]
pub(crate) struct GenerateConfigArgs {
    /// Generates the node config
    #[arg(long, group = "input1")]
    node: bool,
    /// Generates the rpc config
    #[arg(long, group = "input1")]
    rpc: bool,
    /// Uses the default config values
    #[arg(long)]
    use_defaults: bool,
    /// Uses the supplied path as the data directory
    #[arg(long, group = "input2")]
    data_path: Option<String>,
    /// Uses the supplied network (live, test, beta or dev)
    #[arg(long, group = "input2")]
    network: Option<String>,
}

impl GenerateConfigArgs {
    pub(crate) fn generate_config(&self) -> Result<()> {
        let mut toml = TomlConfig::new();
        let network = NetworkConstants::active_network();
        let mut config_type = "node";

        if self.node {
            let network_params = NetworkParams::new(network);
            let mut config = DaemonConfig::new(&network_params, 0)?;
            config.node.peering_port = Some(network_params.network.default_node_port);
            config.serialize_toml(&mut toml)?
        } else {
            config_type = "rpc";
            let network_constants = NetworkConstants::new(WorkThresholds::new(0, 0, 0), network);
            let config = RpcConfig::new(&network_constants, 0);
            config.serialize_toml(&mut toml)?
        }

        //if !self.use_defaults {
        println!("# This is an example configuration file for Nano. Visit https://docs.nano.org/running-a-node/configuration/ for more information.\n#");
        println!("# Fields may need to be defined in the context of a [category] above them.\n");
        println!("# The desired configuration changes should be placed in config-{}.toml in the node data path.\n", config_type);
        println!("# To change a value from its default, uncomment (erasing #) the corresponding field.\n");
        println!("# It is not recommended to uncomment every field, as the default value for important fields may change in the future. Only change what you need.\n");
        println!("# Additional information for notable configuration options is available in https://docs.nano.org/running-a-node/configuration/#notable-configuration-options\n");

        println!("{}", toml.to_string());
        //}

        Ok(())
    }
}
