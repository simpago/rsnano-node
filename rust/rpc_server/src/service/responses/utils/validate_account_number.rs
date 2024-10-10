use rsnano_rpc_messages::SuccessDto;
use serde_json::to_string_pretty;

pub async fn validate_account_number() -> String {
    to_string_pretty(&SuccessDto::new()).unwrap()
}
