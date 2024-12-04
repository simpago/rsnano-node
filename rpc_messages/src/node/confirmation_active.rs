use crate::{RpcCommand, RpcU64};
use rsnano_core::QualifiedRoot;
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn confirmation_active(announcements: Option<u64>) -> Self {
        Self::ConfirmationActive(ConfirmationActiveArgs {
            announcements: announcements.map(|i| i.into()),
        })
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ConfirmationActiveArgs {
    pub announcements: Option<RpcU64>,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ConfirmationActiveResponse {
    pub confirmations: Vec<QualifiedRoot>,
    pub unconfirmed: RpcU64,
    pub confirmed: RpcU64,
}
