use anyhow::Result;
use clap::Parser;
use rsnano_core::Account;

#[derive(Parser)]
pub(crate) struct PublicKeyToAccountArgs {
    /// Converts the public_key into the account
    #[arg(long)]
    public_key: String,
}

impl PublicKeyToAccountArgs {
    pub(crate) fn public_key_to_account(&self) -> Result<()> {
        let account = Account::decode_hex(&self.public_key)?;

        println!("Account: {:?}", account.encode_account());

        Ok(())
    }
}
