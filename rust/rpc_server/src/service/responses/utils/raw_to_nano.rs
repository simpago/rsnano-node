use rsnano_core::{Amount, MXRB_RATIO};
use rsnano_rpc_messages::AmountDto;
use serde_json::to_string_pretty;

pub async fn raw_to_nano(amount: Amount) -> String {
    to_string_pretty(&AmountDto::new(
        "raw".to_string(),
        Amount::nano(amount.number() / *MXRB_RATIO),
    ))
    .unwrap()
}
