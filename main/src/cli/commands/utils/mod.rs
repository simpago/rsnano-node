use crate::cli::CliInfrastructure;
use account_to_public_key::AccountToPublicKeyArgs;
use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use expand_private_key::ExpandPrivateKeyArgs;
use public_key_to_account::PublicKeyToAccountArgs;

pub(crate) mod account_to_public_key;
mod create_key_pair;
pub(crate) mod expand_private_key;
pub(crate) mod public_key_to_account;

#[derive(Subcommand)]
pub(crate) enum UtilsSubcommands {
    /// Converts a <public_key> into the account
    PublicKeyToAccount(PublicKeyToAccountArgs),
    /// Converts an <account> into the public key
    AccountToPublicKey(AccountToPublicKeyArgs),
    /// Expands a <private_key> into the public key and the account
    ExpandPrivateKey(ExpandPrivateKeyArgs),
    /// Generates a adhoc random keypair and prints it to stdout
    CreateKeyPair,
}

#[derive(Parser)]
pub(crate) struct UtilsCommand {
    #[command(subcommand)]
    pub subcommand: Option<UtilsSubcommands>,
}

impl UtilsCommand {
    pub(crate) fn run(&self, infra: &mut CliInfrastructure) -> Result<()> {
        match &self.subcommand {
            Some(UtilsSubcommands::PublicKeyToAccount(args)) => args.public_key_to_account()?,
            Some(UtilsSubcommands::AccountToPublicKey(args)) => args.account_to_public_key()?,
            Some(UtilsSubcommands::ExpandPrivateKey(args)) => args.expand_private_key()?,
            Some(UtilsSubcommands::CreateKeyPair) => create_key_pair::create_key_pair(infra),
            None => UtilsCommand::command().print_long_help()?,
        }
        Ok(())
    }
}
