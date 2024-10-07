use rsnano_core::{BlockSubType, JsonBlock};
use serde::{Deserialize, Serialize};
use crate::RpcCommand;

impl RpcCommand {
    pub fn process(
        subtype: Option<BlockSubType>,
        block: JsonBlock,
        force: Option<bool>,
        watch_work: Option<bool>,
        is_async: Option<bool>,
    ) -> Self {
        Self::Process(ProcessArgs::new(subtype, block, force, watch_work, is_async))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ProcessArgs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype: Option<BlockSubType>,
    pub block: JsonBlock,
    pub force: Option<bool>,
    pub watch_work: Option<bool>,
    #[serde(rename = "async")]
    pub is_async: Option<bool>,
}

impl ProcessArgs {
    pub fn new(
        subtype: Option<BlockSubType>,
        block: JsonBlock,
        force: Option<bool>,
        watch_work: Option<bool>,
        is_async: Option<bool>,
    ) -> Self {
        Self {
            subtype,
            block,
            force,
            watch_work,
            is_async,
        }
    }
}