use crate::cli::get_path;
use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use generate_config::GenerateConfigArgs;
use initialize::InitializeArgs;
use rsnano_core::{Account, Amount, BlockHash, PublicKey, RawKey, SendBlock};
use rsnano_node::{wallets::Wallets, BUILD_INFO, VERSION_STRING};
use rsnano_store_lmdb::LmdbEnv;
use run_daemon::RunDaemonArgs;
use std::{sync::Arc, time::Instant};

pub(crate) mod generate_config;
pub(crate) mod initialize;
pub(crate) mod run_daemon;

#[derive(Subcommand)]
pub(crate) enum NodeSubcommands {
    /// Start node daemon.
    RunDaemon(RunDaemonArgs),
    /// Initialize the data folder, if it is not already initialised.
    ///
    /// This command is meant to be run when the data folder is empty, to populate it with the genesis block.
    Initialize(InitializeArgs),
    /// Run internal diagnostics.
    Diagnostics,
    /// Prints out version.
    Version,
    /// Writes node or rpc configuration to stdout, populated with defaults suitable for this system.
    ///
    /// Pass the configuration type node or rpc.
    /// See also use_defaults.
    GenerateConfig(GenerateConfigArgs),
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
            Some(NodeSubcommands::GenerateConfig(args)) => args.generate_config()?,
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
