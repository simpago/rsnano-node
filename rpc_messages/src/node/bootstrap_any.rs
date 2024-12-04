use rsnano_core::Account;
use serde::{Deserialize, Serialize};

use crate::RpcBool;

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub struct BootstrapAnyArgs {
    pub force: Option<RpcBool>,
    pub id: Option<String>,
    pub account: Option<Account>,
}

impl BootstrapAnyArgs {
    pub fn builder() -> BootstrapAnyArgsBuilder {
        BootstrapAnyArgsBuilder {
            args: BootstrapAnyArgs::default(),
        }
    }
}

pub struct BootstrapAnyArgsBuilder {
    args: BootstrapAnyArgs,
}

impl BootstrapAnyArgsBuilder {
    pub fn force(mut self) -> Self {
        self.args.force = Some(true.into());
        self
    }

    pub fn id(mut self, id: String) -> Self {
        self.args.id = Some(id);
        self
    }

    pub fn build(self) -> BootstrapAnyArgs {
        self.args
    }
}
