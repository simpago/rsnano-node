use rsnano_core::QualifiedRoot;
use serde::{Deserialize, Serialize};
use crate::RpcCommand;

impl RpcCommand {
    pub fn confirmation_active(announcements: Option<u64>) -> Self {
        Self::ConfirmationActive(ConfirmationActiveArgs::new(announcements))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ConfirmationActiveArgs {
    pub announcements: Option<u64>,
}

impl ConfirmationActiveArgs {
    pub fn new(announcements: Option<u64>) -> Self {
        Self { announcements }
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ConfirmationActiveDto {
    pub confirmations: Vec<QualifiedRoot>,
    pub unconfirmed: u64,
    pub confirmed: u64,
}

impl ConfirmationActiveDto {
    pub fn new(confirmations: Vec<QualifiedRoot>, unconfirmed: u64, confirmed: u64) -> Self {
        Self {
            confirmations,
            unconfirmed,
            confirmed,
        }
    }
}
