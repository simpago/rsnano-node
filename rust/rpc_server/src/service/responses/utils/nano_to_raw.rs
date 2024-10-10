use rsnano_core::Amount;
use rsnano_rpc_messages::AmountDto;
use serde_json::to_string_pretty;

pub async fn nano_to_raw(nano: Amount) -> String {
    to_string_pretty(&AmountDto::new(
        "raw".to_string(),
        Amount::raw(nano.number()),
    ))
    .unwrap()
}
