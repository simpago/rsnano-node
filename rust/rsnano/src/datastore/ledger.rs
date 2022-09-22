use crate::{ffi::datastore::BLOCK_OR_PRUNED_EXISTS_CALLBACK, BlockHash};
use std::ffi::c_void;

pub struct Ledger {
    handle: *mut c_void,
}

impl Ledger {
    pub fn new(handle: *mut c_void) -> Self {
        Self { handle }
    }
    pub fn block_or_pruned_exists(&self, block: &BlockHash) -> bool {
        unsafe {
            match BLOCK_OR_PRUNED_EXISTS_CALLBACK {
                Some(f) => f(self.handle, block.as_bytes().as_ptr()),
                None => panic!("BLOCK_OR_PRUNED_EXISTS_CALLBACK missing"),
            }
        }
    }
}
