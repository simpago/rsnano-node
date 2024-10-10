use rsnano_core::{Account, PublicKey, RawKey};
use rsnano_rpc_messages::KeyPairDto;
use serde_json::to_string_pretty;

pub async fn deterministic_key(seed: RawKey, index: u32) -> String {
    let private = rsnano_core::deterministic_key(&seed, index);
    let public: PublicKey = (&private).try_into().unwrap();
    let account = Account::from(public);

    to_string_pretty(&KeyPairDto::new(private, public, account)).unwrap()
}
