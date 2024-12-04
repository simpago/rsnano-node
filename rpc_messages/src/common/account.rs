use rsnano_core::Account;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountArg {
    pub account: Account,
}

impl AccountArg {
    pub fn new(account: Account) -> Self {
        Self { account }
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountResponse {
    pub account: Account,
}

impl AccountResponse {
    pub fn new(account: Account) -> Self {
        Self { account }
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountCandidateArg {
    pub account: String,
}
