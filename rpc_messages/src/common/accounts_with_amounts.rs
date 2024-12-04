use indexmap::IndexMap;
use rsnano_core::{Account, Amount};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountsWithAmountsDto {
    pub accounts: HashMap<Account, Amount>,
}

impl AccountsWithAmountsDto {
    pub fn new(accounts: HashMap<Account, Amount>) -> Self {
        Self { accounts }
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct RepresentativesResponse {
    pub representatives: IndexMap<Account, Amount>,
}

impl RepresentativesResponse {
    pub fn new(representatives: IndexMap<Account, Amount>) -> Self {
        Self { representatives }
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct DelegatorsResponse {
    pub delegators: HashMap<Account, Amount>,
}

impl DelegatorsResponse {
    pub fn new(delegators: HashMap<Account, Amount>) -> Self {
        Self { delegators }
    }
}
