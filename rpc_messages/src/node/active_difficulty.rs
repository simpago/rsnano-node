use crate::RpcF32;
use rsnano_core::WorkNonce;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct ActiveDifficultyResponse {
    pub deprecated: String,
    pub network_minimum: WorkNonce,
    pub network_receive_minimum: WorkNonce,
    pub network_current: WorkNonce,
    pub network_receive_current: WorkNonce,
    pub multiplier: RpcF32,
    pub difficulty_trend: Option<RpcF32>,
}
