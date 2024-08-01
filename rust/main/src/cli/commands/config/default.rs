use anyhow::Result;
use clap::{ArgGroup, Parser};
use rsnano_node::config::DaemonToml;
use rsnano_rpc::RpcToml;
use std::io::BufRead;

#[derive(Parser)]
#[command(group = ArgGroup::new("input1")
    .args(&["node", "rpc"])
    .required(true))]
pub(crate) struct DefaultArgs {
    /// Prints the default node config
    #[arg(long, group = "input1")]
    node: bool,
    /// Prints the default rpc config
    #[arg(long, group = "input1")]
    rpc: bool,
    /// Uncomments the entries of the config
    #[arg(long)]
    use_defaults: bool,
}

impl DefaultArgs {
    pub(crate) fn default(&self) -> Result<()> {
        let (toml_str, config_type) = if self.node {
            let daemon_toml = DaemonToml::default();
            (toml::to_string(&daemon_toml)?, "node")
        } else {
            let rpc_toml = RpcToml::default();
            (toml::to_string(&rpc_toml)?, "rpc")
        };

        println!("# This is an example configuration file for Nano. Visit https://docs.nano.org/running-a-node/configuration/ for more information.");
        println!("# Fields may need to be defined in the context of a [category] above them.");
        println!("# The desired configuration changes should be placed in config-{}.toml in the node data path.", config_type);
        println!(
            "# To change a value from its default, uncomment (erasing #) the corresponding field."
        );
        println!("# It is not recommended to uncomment every field, as the default value for important fields may change in the future. Only change what you need.");
        println!("# Additional information for notable configuration options is available in https://docs.nano.org/running-a-node/configuration/#notable-configuration-options\n");

        if self.use_defaults {
            println!("{}", with_comments(&toml_str, false));
        } else {
            println!("{}", with_comments(&toml_str, true));
        }

        Ok(())
    }
}

fn with_comments(toml_string: &String, comment_values: bool) -> String {
    let mut ss_processed = String::new();

    let reader = std::io::BufReader::new(toml_string.as_bytes());

    for line in reader.lines() {
        let mut line = line.unwrap();
        if !line.is_empty() && !line.starts_with('[') {
            if line.starts_with('#') {
                line = format!("\t{}", line);
            } else {
                line = if comment_values {
                    format!("\t# {}", line)
                } else {
                    format!("\t{}", line)
                };
            }
        }
        ss_processed.push_str(&line);
        ss_processed.push('\n');
    }

    ss_processed
}