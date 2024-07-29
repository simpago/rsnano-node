use super::VoteHandle;
use crate::transport::ChannelHandle;
use num::FromPrimitive;
use rsnano_node::consensus::{VoteProcessor, VoteProcessorConfig};
use std::{ffi::c_void, ops::Deref, sync::Arc};

pub struct VoteProcessorHandle(pub Arc<VoteProcessor>);

impl Deref for VoteProcessorHandle {
    type Target = Arc<VoteProcessor>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub type VoteProcessorVoteProcessedCallback =
    unsafe extern "C" fn(*mut c_void, *mut VoteHandle, *mut ChannelHandle, u8, u8);

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_processor_destroy(handle: *mut VoteProcessorHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub extern "C" fn rsn_vote_processor_vote_blocking(
    handle: &VoteProcessorHandle,
    vote: &VoteHandle,
    channel: &ChannelHandle,
    source: u8,
) -> u8 {
    handle.0.vote_blocking(
        vote,
        &Some(channel.deref().clone()),
        FromPrimitive::from_u8(source).unwrap(),
    ) as u8
}

#[repr(C)]
pub struct VoteProcessorConfigDto {
    pub max_pr_queue: usize,
    pub max_non_pr_queue: usize,
    pub pr_priority: usize,
    pub threads: usize,
    pub batch_size: usize,
    pub max_triggered: usize,
}

impl From<&VoteProcessorConfigDto> for VoteProcessorConfig {
    fn from(value: &VoteProcessorConfigDto) -> Self {
        Self {
            max_pr_queue: value.max_pr_queue,
            max_non_pr_queue: value.max_non_pr_queue,
            pr_priority: value.pr_priority,
            threads: value.threads,
            batch_size: value.batch_size,
            max_triggered: value.max_triggered,
        }
    }
}

impl From<&VoteProcessorConfig> for VoteProcessorConfigDto {
    fn from(value: &VoteProcessorConfig) -> Self {
        Self {
            max_pr_queue: value.max_pr_queue,
            max_non_pr_queue: value.max_non_pr_queue,
            pr_priority: value.pr_priority,
            threads: value.threads,
            batch_size: value.batch_size,
            max_triggered: value.max_triggered,
        }
    }
}
