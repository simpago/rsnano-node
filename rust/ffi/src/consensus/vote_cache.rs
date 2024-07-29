use crate::consensus::VoteHandle;
use rsnano_core::{Amount, BlockHash, Vote, VoteCode};
use rsnano_node::consensus::{VoteCache, VoteCacheConfig};
use std::{
    collections::HashMap,
    ops::Deref,
    sync::{Arc, Mutex},
    time::Duration,
};

pub struct VoteCacheHandle(pub Arc<Mutex<VoteCache>>);

impl Deref for VoteCacheHandle {
    type Target = Arc<Mutex<VoteCache>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_cache_destroy(handle: *mut VoteCacheHandle) {
    drop(Box::from_raw(handle));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_cache_cache_size(handle: *const VoteCacheHandle) -> usize {
    (*handle).0.lock().unwrap().size()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_cache_find(
    handle: *mut VoteCacheHandle,
    hash: *const u8,
) -> *mut VoteVecHandle {
    let hash = BlockHash::from_ptr(hash);
    let guard = (*handle).0.lock().unwrap();
    VoteVecHandle::new(guard.find(&hash))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_cache_clear(handle: *mut VoteCacheHandle) {
    let mut guard = (*handle).0.lock().unwrap();
    guard.clear()
}

pub struct VoteResultMapHandle(Vec<(BlockHash, VoteCode)>);

impl VoteResultMapHandle {
    pub fn new(map: &HashMap<BlockHash, VoteCode>) -> *mut Self {
        Box::into_raw(Box::new(Self(map.iter().map(|(k, v)| (*k, *v)).collect())))
    }
}

impl From<&VoteResultMapHandle> for HashMap<BlockHash, VoteCode> {
    fn from(value: &VoteResultMapHandle) -> Self {
        value.0.iter().map(|(k, v)| (*k, *v)).collect()
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_result_map_destroy(handle: *mut VoteResultMapHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_result_map_len(handle: &VoteResultMapHandle) -> usize {
    handle.0.len()
}
#[no_mangle]

pub unsafe extern "C" fn rsn_vote_result_map_get(
    handle: &VoteResultMapHandle,
    index: usize,
    hash: *mut u8,
) -> u8 {
    let (block_hash, code) = &handle.0[index];
    block_hash.copy_bytes(hash);
    *code as u8
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_cache_vote(
    handle: &mut VoteCacheHandle,
    vote: &VoteHandle,
    weight: *const u8,
) {
    // TODO: pass filter callback too!
    handle
        .0
        .lock()
        .unwrap()
        .insert(vote, Amount::from_ptr(weight), &HashMap::new());
}

#[repr(C)]
pub struct VoteCacheConfigDto {
    pub max_size: usize,
    pub max_voters: usize,
    pub age_cutoff_s: u64,
}

impl From<&VoteCacheConfig> for VoteCacheConfigDto {
    fn from(value: &VoteCacheConfig) -> Self {
        Self {
            max_size: value.max_size,
            max_voters: value.max_voters,
            age_cutoff_s: value.age_cutoff.as_secs(),
        }
    }
}

impl From<&VoteCacheConfigDto> for VoteCacheConfig {
    fn from(value: &VoteCacheConfigDto) -> Self {
        Self {
            max_size: value.max_size,
            max_voters: value.max_voters,
            age_cutoff: Duration::from_secs(value.age_cutoff_s),
        }
    }
}

pub struct VoteVecHandle(Vec<Arc<Vote>>);

impl VoteVecHandle {
    pub fn new(votes: Vec<Arc<Vote>>) -> *mut Self {
        Box::into_raw(Box::new(VoteVecHandle(votes)))
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_vec_destroy(handle: *mut VoteVecHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_vec_len(handle: &VoteVecHandle) -> usize {
    handle.0.len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_vec_get(handle: &VoteVecHandle, index: usize) -> *mut VoteHandle {
    VoteHandle::new(Arc::clone(handle.0.get(index).unwrap()))
}
