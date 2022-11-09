use crate::{block_processing::BlockProcessor, ffi::core::UncheckedInfoHandle};

use std::{ffi::c_void, ops::Deref, sync::Arc};

pub struct BlockProcessorHandle(Arc<BlockProcessor>);

impl Deref for BlockProcessorHandle {
    type Target = Arc<BlockProcessor>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub extern "C" fn rsn_block_processor_create(handle: *mut c_void) -> *mut BlockProcessorHandle {
    let processor = BlockProcessor::new(handle);
    Box::into_raw(Box::new(BlockProcessorHandle(Arc::new(processor))))
}

#[no_mangle]
pub extern "C" fn rsn_block_processor_destroy(handle: *mut BlockProcessorHandle) {
    drop(unsafe { Box::from_raw(handle) });
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_block_processor_add(f: BlockProcessorAddCallback) {
    BLOCKPROCESSOR_ADD_CALLBACK = Some(f);
}

type BlockProcessorAddCallback = unsafe extern "C" fn(*mut c_void, *mut UncheckedInfoHandle);
pub(crate) static mut BLOCKPROCESSOR_ADD_CALLBACK: Option<BlockProcessorAddCallback> = None;
