use crate::core::Amount;
use crate::ffi::{copy_amount_bytes, core::BlockHandle};
use crate::voting::ElectionStatus;
use num_traits::FromPrimitive;
use std::ops::Deref;
use std::ptr;
use std::time::{Duration, UNIX_EPOCH};

pub struct ElectionStatusHandle(pub(crate) ElectionStatus);

impl ElectionStatusHandle {
    pub(crate) fn new(info: ElectionStatus) -> Self {
        Self(info)
    }
}

impl Deref for ElectionStatusHandle {
    type Target = ElectionStatus;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_create() -> *mut ElectionStatusHandle {
    let info = ElectionStatus::default();
    Box::into_raw(Box::new(ElectionStatusHandle(info)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_create1(
    winner: *const BlockHandle,
) -> *mut ElectionStatusHandle {
    let winner = (*winner).block.clone();
    let info = ElectionStatus {
        winner: Some(winner),
        ..Default::default()
    };
    Box::into_raw(Box::new(ElectionStatusHandle(info)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_clone(
    handle: *const ElectionStatusHandle,
) -> *mut ElectionStatusHandle {
    Box::into_raw(Box::new(ElectionStatusHandle((*handle).0.clone())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_destroy(handle: *mut ElectionStatusHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_get_winner(
    handle: *const ElectionStatusHandle,
) -> *mut BlockHandle {
    match (*handle).0.winner.clone() {
        Some(winner) => Box::into_raw(Box::new(BlockHandle::new(winner))),
        None => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_confirmation_request_count(
    handle: *const ElectionStatusHandle,
) -> u32 {
    (*handle).0.confirmation_request_count
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_get_tally(
    handle: *const ElectionStatusHandle,
    result: *mut u8,
) {
    copy_amount_bytes((*handle).0.tally, result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_get_final_tally(
    handle: *const ElectionStatusHandle,
    result: *mut u8,
) {
    copy_amount_bytes((*handle).0.final_tally, result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_get_election_end(
    handle: *const ElectionStatusHandle,
) -> i64 {
    (*handle)
        .0
        .election_end
        .map(|x| x.duration_since(UNIX_EPOCH).unwrap().as_millis() as i64)
        .unwrap_or_default()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_get_election_duration(
    handle: *const ElectionStatusHandle,
) -> i64 {
    (*handle).0.election_duration.as_millis() as i64
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_get_confirmation_request_count(
    handle: *const ElectionStatusHandle,
) -> u32 {
    (*handle).0.confirmation_request_count
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_get_block_count(
    handle: *const ElectionStatusHandle,
) -> u32 {
    (*handle).0.block_count
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_get_vote_count(
    handle: *const ElectionStatusHandle,
) -> u32 {
    (*handle).0.voter_count
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_get_election_status_type(
    handle: *const ElectionStatusHandle,
) -> u8 {
    (*handle).0.election_status_type as u8
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_set_winner(
    handle: *mut ElectionStatusHandle,
    winner: *const BlockHandle,
) {
    (*handle).0.winner = if winner.is_null() {
        None
    } else {
        Some((*winner).block.clone())
    };
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_set_tally(
    handle: *mut ElectionStatusHandle,
    tally: *const u8,
) {
    (*handle).0.tally = Amount::from_ptr(tally);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_set_final_tally(
    handle: *mut ElectionStatusHandle,
    final_tally: *const u8,
) {
    (*handle).0.tally = Amount::from_ptr(final_tally);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_set_confirmation_request_count(
    handle: *mut ElectionStatusHandle,
    confirmation_request_count: u32,
) {
    (*handle).0.confirmation_request_count = confirmation_request_count;
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_set_election_end(
    handle: *mut ElectionStatusHandle,
    election_end: i64,
) {
    (*handle).0.election_end = if election_end == 0 {
        None
    } else {
        UNIX_EPOCH.checked_add(Duration::from_millis(election_end as u64))
    };
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_set_election_duration(
    handle: *mut ElectionStatusHandle,
    election_duration: i64,
) {
    (*handle).0.election_duration = Duration::from_millis(election_duration as u64);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_set_block_count(
    handle: *mut ElectionStatusHandle,
    block_count: u32,
) {
    (*handle).0.block_count = block_count;
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_set_voter_count(
    handle: *mut ElectionStatusHandle,
    voter_count: u32,
) {
    (*handle).0.voter_count = voter_count;
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_set_election_status_type(
    handle: *mut ElectionStatusHandle,
    election_status_type: u8,
) {
    (*handle).0.election_status_type = FromPrimitive::from_u8(election_status_type).unwrap();
}
