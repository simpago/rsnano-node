use account_to_public_key::AccountToPublicKeyArgs;
use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use expand_private_key::ExpandPrivateKeyArgs;
use public_key_to_account::PublicKeyToAccountArgs;
use rsnano_core::{Account, KeyPair};

pub(crate) mod account_to_public_key;
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
    pub(crate) fn run(&self) -> Result<()> {
        match &self.subcommand {
            Some(UtilsSubcommands::PublicKeyToAccount(args)) => args.public_key_to_account()?,
            Some(UtilsSubcommands::AccountToPublicKey(args)) => args.account_to_public_key()?,
            Some(UtilsSubcommands::ExpandPrivateKey(args)) => args.expand_private_key()?,
            Some(UtilsSubcommands::CreateKeyPair) => UtilsCommand::create_key_pair(),
            None => UtilsCommand::command().print_long_help()?,
        }

        Ok(())
    }

    fn create_key_pair() {
        let keypair = KeyPair::new();
        let private_key = keypair.private_key();
        let public_key = keypair.public_key();
        let account = Account::from(public_key).encode_account();

        println!("Private: {:?}", private_key);
        println!("Public: {:?}", public_key);
        println!("Account: {:?}", account);
    }
}
