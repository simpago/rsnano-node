use crate::cli::get_path;
use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use generate_config::DefaultConfigArgs;
use initialize::InitializeArgs;
use rsnano_core::{Account, Amount, BlockHash, PublicKey, RawKey, SendBlock};
use rsnano_node::{wallets::Wallets, BUILD_INFO, VERSION_STRING};
use rsnano_store_lmdb::LmdbEnv;
use run_daemon::RunDaemonArgs;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    sync::Arc,
    time::Instant,
};
use update_config::CurrentConfigArgs;

pub(crate) mod generate_config;
pub(crate) mod initialize;
pub(crate) mod run_daemon;
pub(crate) mod update_config;

#[derive(Subcommand)]
pub(crate) enum NodeSubcommands {
    /// Starts the node daemon
    RunDaemon(RunDaemonArgs),
    /// Initializes the data folder, if it is not already initialised
    Initialize(InitializeArgs),
    /// Runs internal diagnostics
    Diagnostics,
    /// Prints out the version
    Version,
    /// Prints the default configuration
    DefaultConfig(DefaultConfigArgs),
    /// Prints the current configuration
    CurrentConfig(CurrentConfigArgs),
}

#[derive(Parser)]
pub(crate) struct NodeCommand {
    #[command(subcommand)]
    pub subcommand: Option<NodeSubcommands>,
}

impl NodeCommand {
    pub(crate) fn run(&self) -> Result<()> {
        match &self.subcommand {
            Some(NodeSubcommands::RunDaemon(args)) => args.run_daemon()?,
            Some(NodeSubcommands::Initialize(args)) => args.initialize()?,
            Some(NodeSubcommands::DefaultConfig(args)) => args.generate_config()?,
            Some(NodeSubcommands::CurrentConfig(args)) => args.update_config()?,
            Some(NodeSubcommands::Version) => Self::version(),
            Some(NodeSubcommands::Diagnostics) => Self::diagnostics()?,
            None => NodeCommand::command().print_long_help()?,
        }

        Ok(())
    }

    fn version() {
        println!("Version {}", VERSION_STRING);
        println!("Build Info {}", BUILD_INFO);
    }

    fn diagnostics() -> Result<()> {
        let path = get_path(&None, &None).join("wallets.ldb");

        let env = Arc::new(LmdbEnv::new(&path)?);

        let wallets = Wallets::new_null_with_env(env)?;

        println!("Testing hash function");

        SendBlock::new(
            &BlockHash::zero(),
            &Account::zero(),
            &Amount::zero(),
            &RawKey::zero(),
            &PublicKey::zero(),
            0,
        );

        println!("Testing key derivation function");

        wallets.kdf.hash_password("", &mut [0; 32]);

        println!("Testing time retrieval latency...");

        let iters = 2_000_000;
        let start = Instant::now();
        for _ in 0..iters {
            let _ = Instant::now();
        }
        let duration = start.elapsed();
        let avg_duration = duration.as_nanos() as f64 / iters as f64;

        println!("{} nanoseconds", avg_duration);

        Ok(())
    }
}

fn read_node_config_toml(path: &PathBuf) -> Result<String> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Read the file line by line, ignoring lines that start with `#`
    let mut toml_str = String::new();
    for line in reader.lines() {
        let line = line?;
        if !line.trim_start().starts_with('#') {
            toml_str.push_str(&line);
            toml_str.push('\n');
        }
    }
    Ok(toml_str)
}
