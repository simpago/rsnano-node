use super::VoteHandle;
use crate::transport::ChannelHandle;
use num::FromPrimitive;
use rsnano_node::consensus::VoteProcessorQueue;
use std::{ops::Deref, sync::Arc};

pub struct VoteProcessorQueueHandle(pub Arc<VoteProcessorQueue>);

impl Deref for VoteProcessorQueueHandle {
    type Target = Arc<VoteProcessorQueue>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_processor_queue_destroy(handle: *mut VoteProcessorQueueHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub extern "C" fn rsn_vote_processor_queue_vote(
    handle: &VoteProcessorQueueHandle,
    vote: &VoteHandle,
    channel: &ChannelHandle,
    source: u8,
) -> bool {
    handle.0.vote(
        (**vote).clone(),
        channel.channel_id(),
        FromPrimitive::from_u8(source).unwrap(),
    )
}
