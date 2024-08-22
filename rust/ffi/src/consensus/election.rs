use crate::{
    core::{copy_block_array_dto, BlockArrayDto, BlockHandle},
    utils::ContextWrapper,
    VoidPointerCallback,
};
use num_traits::FromPrimitive;
use rsnano_core::{utils::system_time_as_nanoseconds, BlockEnum, BlockHash, PublicKey};
use rsnano_node::consensus::{
    Election, ElectionBehavior, ElectionData, ElectionState, VoteInfo, NEXT_ELECTION_ID,
};
use std::{
    ffi::c_void,
    ops::Deref,
    sync::{atomic::Ordering, Arc, MutexGuard},
    time::{Duration, SystemTime},
};

use super::election_status::ElectionStatusHandle;

pub struct ElectionHandle(pub Arc<Election>);

impl ElectionHandle {
    pub fn new(election: Arc<Election>) -> *mut ElectionHandle {
        Box::into_raw(Box::new(Self(election)))
    }
}

impl Deref for ElectionHandle {
    type Target = Arc<Election>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub type ConfirmationAction = unsafe extern "C" fn(*mut c_void, *mut BlockHandle);
pub type LiveVoteAction = unsafe extern "C" fn(*mut c_void, *const u8);

#[no_mangle]
pub unsafe extern "C" fn rsn_election_create(
    block: &BlockHandle,
    behavior: u8,
    confirmation_action: ConfirmationAction,
    confirmation_action_context: *mut c_void,
    confirmation_action_context_delete: VoidPointerCallback,
    live_vote_action: LiveVoteAction,
    live_vote_action_context: *mut c_void,
    live_vote_action_context_delete: VoidPointerCallback,
) -> *mut ElectionHandle {
    let confirmation_context = ContextWrapper::new(
        confirmation_action_context,
        confirmation_action_context_delete,
    );

    let live_vote_context =
        ContextWrapper::new(live_vote_action_context, live_vote_action_context_delete);
    let id = NEXT_ELECTION_ID.fetch_add(1, Ordering::Relaxed);

    ElectionHandle::new(Arc::new(Election::new(
        id,
        Arc::clone(block),
        ElectionBehavior::from_u8(behavior).unwrap(),
        Box::new(move |block| {
            confirmation_action(
                confirmation_context.get_context(),
                Box::into_raw(Box::new(BlockHandle(block))),
            );
        }),
        Box::new(move |account| {
            live_vote_action(live_vote_context.get_context(), account.as_bytes().as_ptr())
        }),
    )))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_destroy(handle: *mut ElectionHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub extern "C" fn rsn_election_lock(handle: &ElectionHandle) -> *mut ElectionLockHandle {
    let guard = handle.mutex.lock().unwrap();
    let guard = unsafe {
        std::mem::transmute::<MutexGuard<ElectionData>, MutexGuard<'static, ElectionData>>(guard)
    };
    Box::into_raw(Box::new(ElectionLockHandle(Some(guard))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_qualified_root(
    handle: &ElectionHandle,
    root: *mut u8,
    previous: *mut u8,
) {
    handle.qualified_root.root.copy_bytes(root);
    handle.qualified_root.previous.copy_bytes(previous);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_confirmation_request_count(handle: &ElectionHandle) -> u32 {
    handle.0.confirmation_request_count.load(Ordering::SeqCst)
}

#[no_mangle]
pub extern "C" fn rsn_election_behavior(handle: &ElectionHandle) -> u8 {
    handle.0.behavior as u8
}

pub struct ElectionLockHandle(pub Option<MutexGuard<'static, ElectionData>>);

#[no_mangle]
pub unsafe extern "C" fn rsn_election_lock_destroy(handle: *mut ElectionLockHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub extern "C" fn rsn_election_lock_status(
    handle: &ElectionLockHandle,
) -> *mut ElectionStatusHandle {
    Box::into_raw(Box::new(ElectionStatusHandle(
        handle.0.as_ref().unwrap().status.clone(),
    )))
}

#[no_mangle]
pub extern "C" fn rsn_election_lock_state_change(
    handle: &mut ElectionLockHandle,
    expected_state: u8,
    desired_state: u8,
) -> bool {
    let expected = ElectionState::from_u8(expected_state).unwrap();
    let desired = ElectionState::from_u8(desired_state).unwrap();
    handle
        .0
        .as_mut()
        .unwrap()
        .state_change(expected, desired)
        .is_err()
}

#[no_mangle]
pub extern "C" fn rsn_election_lock_state_start_elapsed_ms(handle: &ElectionLockHandle) -> u64 {
    handle.0.as_ref().unwrap().state_start.elapsed().as_millis() as u64
}

#[no_mangle]
pub extern "C" fn rsn_election_lock_state(handle: &ElectionLockHandle) -> u8 {
    handle.0.as_ref().unwrap().state as u8
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_lock_blocks_size(handle: &ElectionLockHandle) -> usize {
    handle.0.as_ref().unwrap().last_blocks.len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_lock_blocks_find(
    handle: &ElectionLockHandle,
    hash: *const u8,
) -> *mut BlockHandle {
    match handle
        .0
        .as_ref()
        .unwrap()
        .last_blocks
        .get(&BlockHash::from_ptr(hash))
    {
        Some(block) => Box::into_raw(Box::new(BlockHandle(Arc::clone(block)))),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_lock_blocks(
    handle: &ElectionLockHandle,
    result: &mut BlockArrayDto,
) {
    let blocks: Vec<Arc<BlockEnum>> = handle
        .0
        .as_ref()
        .unwrap()
        .last_blocks
        .values()
        .cloned()
        .collect();

    copy_block_array_dto(blocks, result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_lock_votes_insert(
    handle: &mut ElectionLockHandle,
    account: *const u8,
    vote: &VoteInfoHandle,
) {
    handle
        .0
        .as_mut()
        .unwrap()
        .last_votes
        .insert(PublicKey::from_ptr(account), vote.0.clone());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_lock_votes_find(
    handle: &ElectionLockHandle,
    account: *const u8,
) -> *mut VoteInfoHandle {
    match handle
        .0
        .as_ref()
        .unwrap()
        .last_votes
        .get(&PublicKey::from_ptr(account))
    {
        Some(info) => VoteInfoHandle::new(info.clone()),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_lock_votes_size(handle: &ElectionLockHandle) -> usize {
    handle.0.as_ref().unwrap().last_votes.len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_lock_votes(
    handle: &ElectionLockHandle,
) -> *mut VoteInfoCollectionHandle {
    let votes = handle
        .0
        .as_ref()
        .unwrap()
        .last_votes
        .iter()
        .map(|(a, i)| (*a, i.clone()))
        .collect::<Vec<_>>();

    Box::into_raw(Box::new(VoteInfoCollectionHandle(votes)))
}

pub struct VoteInfoCollectionHandle(Vec<(PublicKey, VoteInfo)>);

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_info_collection_destroy(handle: *mut VoteInfoCollectionHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub extern "C" fn rsn_vote_info_collection_len(handle: &VoteInfoCollectionHandle) -> usize {
    handle.0.len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_info_collection_get(
    handle: &VoteInfoCollectionHandle,
    index: usize,
    account: *mut u8,
) -> *mut VoteInfoHandle {
    let (acc, vote) = &handle.0[index];
    acc.copy_bytes(account);
    return VoteInfoHandle::new(vote.clone());
}

pub struct VoteInfoHandle(VoteInfo);

impl VoteInfoHandle {
    pub fn new(info: VoteInfo) -> *mut VoteInfoHandle {
        Box::into_raw(Box::new(VoteInfoHandle(info)))
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_info_create1() -> *mut VoteInfoHandle {
    VoteInfoHandle::new(Default::default())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_info_create2(
    timestamp: u64,
    hash: *const u8,
) -> *mut VoteInfoHandle {
    VoteInfoHandle::new(VoteInfo {
        time: SystemTime::now(),
        timestamp,
        hash: BlockHash::from_ptr(hash),
    })
}

#[no_mangle]
pub extern "C" fn rsn_vote_info_clone(handle: &VoteInfoHandle) -> *mut VoteInfoHandle {
    VoteInfoHandle::new(handle.0.clone())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_info_destroy(handle: *mut VoteInfoHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub extern "C" fn rsn_vote_info_time_ns(handle: &VoteInfoHandle) -> u64 {
    system_time_as_nanoseconds(handle.0.time)
}

#[no_mangle]
pub extern "C" fn rsn_vote_info_timestamp(handle: &VoteInfoHandle) -> u64 {
    handle.0.timestamp
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_info_hash(handle: &VoteInfoHandle, hash: *mut u8) {
    handle.0.hash.copy_bytes(hash);
}

#[no_mangle]
pub extern "C" fn rsn_vote_info_with_relative_time(
    handle: &VoteInfoHandle,
    seconds: i64,
) -> *mut VoteInfoHandle {
    let delta = Duration::from_secs(seconds.abs() as u64);
    VoteInfoHandle::new(VoteInfo {
        time: if seconds < 0 {
            SystemTime::now() - delta
        } else {
            SystemTime::now() + delta
        },
        timestamp: handle.0.timestamp,
        hash: handle.0.hash,
    })
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_contains(handle: &ElectionHandle, hash: *const u8) -> bool {
    handle.0.contains(&BlockHash::from_ptr(hash))
}
