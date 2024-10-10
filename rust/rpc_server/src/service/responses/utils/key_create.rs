use rsnano_core::{Account, KeyPair};
use rsnano_rpc_messages::KeyPairDto;
use serde_json::to_string_pretty;

pub async fn key_create() -> String {
    let keypair = KeyPair::new();
    let private = keypair.private_key();
    let public = keypair.public_key();
    let account = Account::from(public);

    to_string_pretty(&KeyPairDto::new(private, public, account)).unwrap()
}
