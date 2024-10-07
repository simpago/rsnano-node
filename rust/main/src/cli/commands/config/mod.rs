use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use current::CurrentArgs;
use default::DefaultArgs;

pub(crate) mod current;
pub(crate) mod default;

#[derive(Subcommand)]
pub(crate) enum ConfigSubcommands {
    /// Prints the default configs.
    Default(DefaultArgs),
    /// Prints the current configs
    Current(CurrentArgs),
}

#[derive(Parser)]
pub(crate) struct ConfigCommand {
    #[command(subcommand)]
    pub subcommand: Option<ConfigSubcommands>,
}

impl ConfigCommand {
    pub(crate) fn run(&self) -> Result<()> {
        match &self.subcommand {
            Some(ConfigSubcommands::Default(args)) => args.default()?,
            Some(ConfigSubcommands::Current(args)) => args.current()?,
            None => ConfigCommand::command().print_long_help()?,
        }

        Ok(())
    }
}
