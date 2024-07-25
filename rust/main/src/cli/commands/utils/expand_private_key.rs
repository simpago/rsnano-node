use anyhow::Result;
use clap::Parser;
use rsnano_core::{Account, PublicKey, RawKey};

#[derive(Parser)]
pub(crate) struct ExpandPrivateKeyArgs {
    /// Derives the public key and the account from the <private_key>
    #[arg(long)]
    private_key: String,
}

impl ExpandPrivateKeyArgs {
    pub(crate) fn expand_private_key(&self) -> Result<()> {
        let private_key = RawKey::decode_hex(&self.private_key)?;
        let public_key = PublicKey::try_from(&private_key)?;
        let account = Account::encode_account(&public_key);

        println!("Private: {:?}", private_key);
        println!("Public: {:?}", public_key);
        println!("Account: {:?}", account);

        Ok(())
    }
}
