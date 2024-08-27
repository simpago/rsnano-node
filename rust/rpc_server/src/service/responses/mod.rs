mod ledger;
mod node;
mod utils;
mod wallets;

pub use ledger::*;
pub use node::*;
use serde_json::{json, to_string_pretty};
pub use utils::*;
pub use wallets::*;

fn format_error_message(error: &str) -> String {
    let json_value = json!({ "error": error });
    to_string_pretty(&json_value).unwrap()
}
