use crate::RpcCommand;
use rsnano_core::Account;
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn bootstrap_any(
        force: Option<bool>,
        id: Option<String>,
        account: Option<Account>,
    ) -> Self {
        Self::BootstrapAny(BootstrapAnyArgs::new(force, id, account))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]

pub struct BootstrapAnyArgs {
    pub force: Option<bool>,
    pub id: Option<String>,
    pub account: Option<Account>,
}

impl BootstrapAnyArgs {
    pub fn new(force: Option<bool>, id: Option<String>, account: Option<Account>) -> Self {
        Self { force, id, account }
    }
}
