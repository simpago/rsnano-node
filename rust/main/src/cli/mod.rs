use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use commands::{
    ledger::LedgerCommand, node::NodeCommand, utils::UtilsCommand, wallets::WalletsCommand,
};
use rsnano_core::Networks;
use rsnano_node::{config::NetworkConstants, working_path};
use std::{path::PathBuf, str::FromStr};
use tracing_subscriber::EnvFilter;

mod commands;

#[derive(Parser)]
#[command(name = "rsnano_node")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

impl Cli {
    pub(crate) fn run(&self) -> Result<()> {
        match &self.command {
            Some(Commands::Wallets(command)) => command.run()?,
            Some(Commands::Utils(command)) => command.run()?,
            Some(Commands::Node(command)) => command.run()?,
            Some(Commands::Ledger(command)) => command.run()?,
            None => Cli::command().print_long_help()?,
        }
        Ok(())
    }
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Commands related to the ledger
    Ledger(LedgerCommand),
    /// Commands related to running the node
    Node(NodeCommand),
    /// Utils related to keys and accounts
    Utils(UtilsCommand),
    /// Commands to manage wallets
    Wallets(WalletsCommand),
}

pub(crate) fn get_path(path_str: &Option<String>, network_str: &Option<String>) -> PathBuf {
    if let Some(path) = path_str {
        return PathBuf::from_str(path).unwrap();
    }
    if let Some(network) = network_str {
        let network = Networks::from_str(&network).unwrap();
        NetworkConstants::set_active_network(network);
    }
    working_path().unwrap()
}

pub(crate) fn init_tracing(dirs: impl AsRef<str>) {
    let filter = EnvFilter::builder().parse_lossy(dirs);
    let value = std::env::var("NANO_LOG");
    let log_style = value.as_ref().map(|i| i.as_str()).unwrap_or_default();
    match log_style {
        "json" => {
            tracing_subscriber::fmt::fmt()
                .json()
                .with_env_filter(filter)
                .init();
        }
        "noansi" => {
            tracing_subscriber::fmt::fmt()
                .with_env_filter(filter)
                .with_ansi(false)
                .init();
        }
        _ => {
            tracing_subscriber::fmt::fmt()
                .with_env_filter(filter)
                .with_ansi(true)
                .init();
        }
    }
    tracing::debug!(log_style, ?value, "init tracing");
}
