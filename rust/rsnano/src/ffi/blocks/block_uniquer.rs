use super::BlockHandle;
use crate::BlockUniquer;
use std::{ops::Deref, sync::Arc};

pub struct BlockUniquerHandle(Arc<BlockUniquer>);

impl Deref for BlockUniquerHandle {
    type Target = Arc<BlockUniquer>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub extern "C" fn rsn_block_uniquer_create() -> *mut BlockUniquerHandle {
    Box::into_raw(Box::new(BlockUniquerHandle(Arc::new(BlockUniquer::new()))))
}

#[no_mangle]
pub extern "C" fn rsn_block_uniquer_destroy(handle: *mut BlockUniquerHandle) {
    let uniquer = unsafe { Box::from_raw(handle) };
    drop(uniquer);
}

#[no_mangle]
pub extern "C" fn rsn_block_uniquer_size(handle: *const BlockUniquerHandle) -> usize {
    unsafe { &*handle }.0.size()
}

#[no_mangle]
pub extern "C" fn rsn_block_uniquer_unique(
    handle: *mut BlockUniquerHandle,
    block: *mut BlockHandle,
) -> *mut BlockHandle {
    let original = &unsafe { &*block }.block;
    let uniqued = unsafe { &*handle }.0.unique(original);
    if Arc::ptr_eq(&uniqued, original) {
        block
    } else {
        Box::into_raw(Box::new(BlockHandle { block: uniqued }))
    }
}
