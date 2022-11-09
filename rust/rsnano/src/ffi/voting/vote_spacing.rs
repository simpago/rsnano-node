use std::time::Duration;

use crate::{
    core::{BlockHash, Root},
    voting::VoteSpacing,
};

pub struct VoteSpacingHandle(VoteSpacing);

#[no_mangle]
pub extern "C" fn rsn_vote_spacing_create(delay_ms: u64) -> *mut VoteSpacingHandle {
    Box::into_raw(Box::new(VoteSpacingHandle(VoteSpacing::new(
        Duration::from_millis(delay_ms),
    ))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_spacing_destroy(handle: *mut VoteSpacingHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_spacing_votable(
    handle: *mut VoteSpacingHandle,
    root: *const u8,
    hash: *const u8,
) -> bool {
    (*handle)
        .0
        .votable(&Root::from_ptr(root), &BlockHash::from_ptr(hash))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_spacing_flag(
    handle: *mut VoteSpacingHandle,
    root: *const u8,
    hash: *const u8,
) {
    (*handle)
        .0
        .flag(&Root::from_ptr(root), &BlockHash::from_ptr(hash));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_spacing_len(handle: *mut VoteSpacingHandle) -> usize {
    (*handle).0.len()
}
