use std::ops::Deref;
use std::slice;
use std::sync::{Arc, RwLock};
use num_traits::FromPrimitive;
use crate::core::{Account, AccountInfo, Amount, BlockHash, Epoch, UncheckedInfo};
use crate::election_status::ElectionStatus;
use crate::ElectionStatusType;
use crate::ffi::core::{AccountInfoHandle, BlockHandle, UncheckedInfoHandle};
use crate::ffi::ledger::{LedgerConstantsDto, RepWeightsHandle};
use crate::ledger::RepWeights;

pub struct ElectionStatusHandle(ElectionStatus);

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
pub extern "C" fn rsn_election_status_create() -> *mut ElectionStatusHandle {
    let info = ElectionStatus::null();
    Box::into_raw(Box::new(ElectionStatusHandle(info)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_create2(
    winner: *const BlockHandle,
    tally: *const u8,
    final_tally: *const u8,
    confirmation_request_count: u32,
    block_count: u32,
    voter_count: u32,
    election_end: i64,
    election_duration: i64,
    election_status_type: u8,
) -> *mut ElectionStatusHandle {
    let winner = (*winner).block.clone();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(std::slice::from_raw_parts(tally, 16));
    let tally = Amount::from_be_bytes(bytes);
    bytes = [0; 16];
    bytes.copy_from_slice(std::slice::from_raw_parts(final_tally, 16));
    let final_tally = Amount::from_be_bytes(bytes);
    let info = ElectionStatus::new(winner, &tally, &final_tally, confirmation_request_count, block_count, voter_count, election_end, election_duration, FromPrimitive::from_u8(election_status_type).unwrap());
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
    Box::into_raw(Box::new(BlockHandle {
        block: (*handle).0.winner.as_ref().unwrap().clone(),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_confirmation_request_count(handle: *const ElectionStatusHandle) -> u32 {
    (*handle).0.confirmation_request_count
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_get_tally(
    handle: *const ElectionStatusHandle,
    result: *mut u8,
) {
    std::slice::from_raw_parts_mut(result, 16).copy_from_slice(&(*handle).0.tally.to_be_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_get_final_tally(
    handle: *const ElectionStatusHandle,
    result: *mut u8,
) {
    std::slice::from_raw_parts_mut(result, 16).copy_from_slice(&(*handle).0.final_tally.to_be_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_get_election_end(handle: *const ElectionStatusHandle) -> i64 {
    (*handle).0.election_end
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_get_election_duration(handle: *const ElectionStatusHandle) -> i64 {
    (*handle).0.election_duration
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_get_confirmation_request_count(handle: *const ElectionStatusHandle) -> u32 {
    (*handle).0.confirmation_request_count
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_get_block_count(handle: *const ElectionStatusHandle) -> u32 {
    (*handle).0.block_count
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_get_vote_count(handle: *const ElectionStatusHandle) -> u32 {
    (*handle).0.voter_count
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_status_get_election_status_type(handle: *const ElectionStatusHandle) -> u8 {
    (*handle).0.election_status_type as u8
}