use std::{ops::Deref, sync::Arc};

use super::vote::VoteHandle;
use crate::voting::VoteUniquer;

pub struct VoteUniquerHandle(Arc<VoteUniquer>);

impl Deref for VoteUniquerHandle {
    type Target = Arc<VoteUniquer>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub extern "C" fn rsn_vote_uniquer_create() -> *mut VoteUniquerHandle {
    Box::into_raw(Box::new(VoteUniquerHandle(Arc::new(VoteUniquer::new()))))
}

#[no_mangle]
pub extern "C" fn rsn_vote_uniquer_destroy(handle: *mut VoteUniquerHandle) {
    let uniquer = unsafe { Box::from_raw(handle) };
    drop(uniquer);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_uniquer_size(handle: *const VoteUniquerHandle) -> usize {
    (*handle).0.size()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_uniquer_unique(
    handle: *mut VoteUniquerHandle,
    vote: *mut VoteHandle,
) -> *mut VoteHandle {
    let original = &*vote;
    let uniqued = (*handle).unique(original);
    if Arc::ptr_eq(&uniqued, original) {
        vote
    } else {
        Box::into_raw(Box::new(VoteHandle::new(uniqued)))
    }
}
