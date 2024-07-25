use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clear::ClearCommand;
use info::InfoCommand;
use snapshot::SnapshotArgs;
use vacuum::VacuumArgs;

pub(crate) mod clear;
pub(crate) mod info;
pub(crate) mod snapshot;
pub(crate) mod vacuum;

#[derive(Subcommand)]
pub(crate) enum LedgerSubcommands {
    /// Commands that get some info from the ledger
    Info(InfoCommand),
    /// Commands that clear some component of the ledger
    Clear(ClearCommand),
    /// Compacts the database
    Vacuum(VacuumArgs),
    /// Similar to vacuum but does not replace the existing database
    Snapshot(SnapshotArgs),
}

#[derive(Parser)]
pub(crate) struct LedgerCommand {
    #[command(subcommand)]
    pub subcommand: Option<LedgerSubcommands>,
}

impl LedgerCommand {
    pub(crate) fn run(&self) -> Result<()> {
        match &self.subcommand {
            Some(LedgerSubcommands::Info(command)) => command.run()?,
            Some(LedgerSubcommands::Clear(command)) => command.run()?,
            Some(LedgerSubcommands::Vacuum(args)) => args.vacuum()?,
            Some(LedgerSubcommands::Snapshot(args)) => args.snapshot()?,
            None => LedgerCommand::command().print_long_help()?,
        }

        Ok(())
    }
}
