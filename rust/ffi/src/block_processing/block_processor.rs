use crate::{core::BlockHandle, transport::ChannelHandle};
use num_traits::FromPrimitive;
use rsnano_node::{
    block_processing::{BlockProcessor, BlockSource},
    transport::ChannelId,
};
use std::{ops::Deref, sync::Arc};

pub struct BlockProcessorHandle(pub Arc<BlockProcessor>);

impl Deref for BlockProcessorHandle {
    type Target = Arc<BlockProcessor>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub extern "C" fn rsn_block_processor_destroy(handle: *mut BlockProcessorHandle) {
    drop(unsafe { Box::from_raw(handle) });
}

#[no_mangle]
pub extern "C" fn rsn_block_processor_stop(handle: &BlockProcessorHandle) {
    handle.stop();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_processor_add(
    handle: &mut BlockProcessorHandle,
    block: &BlockHandle,
    source: u8,
    channel: *const ChannelHandle,
) -> bool {
    let channel = if channel.is_null() {
        None
    } else {
        Some(Arc::clone(&*channel))
    };
    handle.add(
        Arc::clone(block),
        FromPrimitive::from_u8(source).unwrap(),
        channel
            .map(|c| c.channel_id())
            .unwrap_or(ChannelId::LOOPBACK),
    )
}

#[no_mangle]
pub extern "C" fn rsn_block_processor_add_blocking(
    handle: &mut BlockProcessorHandle,
    block: &BlockHandle,
    source: u8,
    status: &mut u8,
) -> bool {
    match handle.add_blocking(Arc::clone(block), BlockSource::from_u8(source).unwrap()) {
        Some(i) => {
            *status = i as u8;
            true
        }
        None => false,
    }
}

#[no_mangle]
pub extern "C" fn rsn_block_processor_force(
    handle: &mut BlockProcessorHandle,
    block: &BlockHandle,
) {
    handle.force(Arc::clone(block));
}
