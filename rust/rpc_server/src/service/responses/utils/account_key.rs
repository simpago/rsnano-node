use rsnano_core::Account;
use rsnano_rpc_messages::KeyRpcMessage;
use serde_json::to_string_pretty;

pub async fn account_key(account: Account) -> String {
    to_string_pretty(&KeyRpcMessage::new(account.into())).unwrap()
}
