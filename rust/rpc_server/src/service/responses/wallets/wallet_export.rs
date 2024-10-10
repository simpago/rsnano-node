use rsnano_core::WalletId;
use rsnano_rpc_messages::JsonDto;
use serde_json::{to_string, to_string_pretty, Value};

pub async fn wallet_export(wallet: WalletId) -> String {
    to_string_pretty(&JsonDto::new(Value::String(to_string(&wallet).unwrap()))).unwrap()
}
