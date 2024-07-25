use anyhow::Result;
use clap::Parser;
use rsnano_core::Account;

#[derive(Parser)]
pub(crate) struct AccountToPublicKeyArgs {
    /// Converts the <account> into the public key
    #[arg(long)]
    account: String,
}

impl AccountToPublicKeyArgs {
    pub(crate) fn account_to_public_key(&self) -> Result<()> {
        let public_key = Account::decode_account(&self.account)?;

        println!("Public key: {:?}", public_key);

        Ok(())
    }
}
