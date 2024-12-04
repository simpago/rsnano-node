use rsnano_core::Account;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use tracing::warn;

#[derive(Clone)]
pub struct VoteOptions {
    representatives: HashSet<String>,
    include_replays: bool,
    include_indeterminate: bool,
}

#[derive(Deserialize)]
pub struct VoteJsonOptions {
    include_replays: Option<bool>,
    include_indeterminate: Option<bool>,
    representatives: Option<Vec<String>>,
}

impl VoteOptions {
    pub fn new(options_a: VoteJsonOptions) -> Self {
        let mut result = Self {
            representatives: HashSet::new(),
            include_replays: false,
            include_indeterminate: false,
        };

        result.include_replays = options_a.include_replays.unwrap_or(false);
        result.include_indeterminate = options_a.include_indeterminate.unwrap_or(false);
        if let Some(representatives_l) = options_a.representatives {
            for representative_l in representatives_l {
                match Account::decode_account(&representative_l) {
                    Ok(result_l) => {
                        // Do not insert the given raw data to keep old prefix support
                        result.representatives.insert(result_l.encode_account());
                    }
                    Err(_) => {
                        warn!(
                            "Invalid account provided for filtering votes: {}",
                            representative_l
                        );
                    }
                }
            }
            // Warn the user if the option will be ignored
            if result.representatives.is_empty() {
                warn!("Account filter for votes is empty, no messages will be filtered");
            }
        }

        result
    }

    /**
     * Checks if a message should be filtered for given vote received options.
     * @param message_a the message to be checked
     * @return false if the message should be broadcasted, true if it should be filtered
     */
    pub fn should_filter(&self, contents: &Value) -> bool {
        let msg_type = match contents.get("type") {
            Some(serde_json::Value::String(s)) => s.as_str(),
            _ => "",
        };

        let mut should_filter_l = (!self.include_replays && msg_type == "replay")
            || (!self.include_indeterminate && msg_type == "indeterminate");

        if !should_filter_l && !self.representatives.is_empty() {
            let representative_text_l = match contents.get("account") {
                Some(serde_json::Value::String(s)) => s.as_str(),
                _ => "",
            };

            if !self.representatives.contains(representative_text_l) {
                should_filter_l = true;
            }
        }
        should_filter_l
    }
}
