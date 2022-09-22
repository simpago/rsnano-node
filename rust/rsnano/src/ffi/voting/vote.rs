use std::{
    ffi::c_void,
    ops::Deref,
    sync::{Arc, RwLock},
};

use crate::{Account, BlockHash, FullHash, RawKey, Signature, Vote};

use crate::ffi::{FfiPropertyTreeWriter, FfiStream, StringDto};

pub struct VoteHandle(Arc<RwLock<Vote>>);

impl VoteHandle {
    pub fn new(vote: Arc<RwLock<Vote>>) -> Self {
        Self(vote)
    }
}

impl Deref for VoteHandle {
    type Target = Arc<RwLock<Vote>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub extern "C" fn rsn_vote_create() -> *mut VoteHandle {
    Box::into_raw(Box::new(VoteHandle::new(Arc::new(RwLock::new(
        Vote::null(),
    )))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_create2(
    account: *const u8,
    prv_key: *const u8,
    timestamp: u64,
    duration: u8,
    hashes: *const [u8; 32],
    hash_count: usize,
) -> *mut VoteHandle {
    let account = Account::from_ptr(account);
    let key = RawKey::from_ptr(prv_key);

    let hashes = std::slice::from_raw_parts(hashes, hash_count);
    let hashes = hashes.iter().map(|&h| BlockHash::from_bytes(h)).collect();

    Box::into_raw(Box::new(VoteHandle::new(Arc::new(RwLock::new(
        Vote::new(account, &key, timestamp, duration, hashes).unwrap(),
    )))))
}

#[no_mangle]
pub extern "C" fn rsn_vote_destroy(handle: *mut VoteHandle) {
    drop(unsafe { Box::from_raw(handle) })
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_copy(handle: *const VoteHandle) -> *mut VoteHandle {
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    let lk = (*handle).read().unwrap();
    Box::into_raw(Box::new(VoteHandle::new(Arc::new(RwLock::new(lk.clone())))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_account(handle: *const VoteHandle, result: *mut u8) {
    let lk = (*handle).read().unwrap();
    let result = std::slice::from_raw_parts_mut(result, 32);
    result.copy_from_slice(lk.voting_account.as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_account_set(handle: *mut VoteHandle, account: *const u8) {
    let mut lk = (*handle).write().unwrap();
    let mut bytes = [0; 32];
    bytes.copy_from_slice(std::slice::from_raw_parts(account, 32));
    lk.voting_account = Account::from_bytes(bytes);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_signature(handle: *const VoteHandle, result: *mut u8) {
    let lk = (*handle).read().unwrap();
    let result = std::slice::from_raw_parts_mut(result, 64);
    result.copy_from_slice(lk.signature.as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_signature_set(handle: *mut VoteHandle, signature: *const u8) {
    let mut lk = (*handle).write().unwrap();
    let mut bytes = [0; 64];
    bytes.copy_from_slice(std::slice::from_raw_parts(signature, 64));
    lk.signature = Signature::from_bytes(bytes);
}

#[repr(C)]
pub struct VoteHashesDto {
    pub handle: *mut VoteHashesHandle,
    pub count: usize,
    pub hashes: *const [u8; 32],
}

pub struct VoteHashesHandle(Vec<[u8; 32]>);

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_hashes(handle: *const VoteHandle) -> VoteHashesDto {
    let hashes: Vec<_> = (*handle)
        .read()
        .unwrap()
        .hashes
        .iter()
        .map(|i| *i.as_bytes())
        .collect();

    let hashes_ptr = hashes.as_ptr();
    let count = hashes.len();
    let handle = Box::into_raw(Box::new(VoteHashesHandle(hashes)));
    VoteHashesDto {
        handle,
        count,
        hashes: hashes_ptr,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_timestamp(handle: *const VoteHandle) -> u64 {
    (*handle).read().unwrap().timestamp()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_duration_bits(handle: *const VoteHandle) -> u8 {
    (*handle).read().unwrap().duration_bits()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_duration_ms(handle: *const VoteHandle) -> u64 {
    (*handle).read().unwrap().duration().as_millis() as u64
}

#[no_mangle]
pub extern "C" fn rsn_vote_hashes_destroy(hashes: *mut VoteHashesHandle) {
    drop(unsafe { Box::from_raw(hashes) });
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_equals(
    first: *const VoteHandle,
    second: *const VoteHandle,
) -> bool {
    if first.is_null() && second.is_null() {
        return true;
    }

    if first.is_null() || second.is_null() {
        return false;
    }

    (*first).read().unwrap().eq(&(*second).read().unwrap())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_hashes_string(handle: *const VoteHandle) -> StringDto {
    (*handle).read().unwrap().vote_hashes_string().into()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_serialize_json(handle: *const VoteHandle, ptree: *mut c_void) {
    let mut writer = FfiPropertyTreeWriter::new_borrowed(ptree);
    (*handle)
        .read()
        .unwrap()
        .serialize_json(&mut writer)
        .unwrap();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_hash(handle: *const VoteHandle, result: *mut u8) {
    let hash = (*handle).read().unwrap().hash();
    let result = std::slice::from_raw_parts_mut(result, 32);
    result.copy_from_slice(hash.as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_full_hash(handle: *const VoteHandle, result: *mut u8) {
    let hash = (*handle).read().unwrap().full_hash();
    let result = std::slice::from_raw_parts_mut(result, 32);
    result.copy_from_slice(hash.as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_serialize(handle: *const VoteHandle, stream: *mut c_void) -> i32 {
    let mut stream = FfiStream::new(stream);
    match (*handle).read().unwrap().serialize(&mut stream) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_deserialize(
    handle: *const VoteHandle,
    stream: *mut c_void,
) -> i32 {
    let mut stream = FfiStream::new(stream);
    match (*handle).write().unwrap().deserialize(&mut stream) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_validate(handle: *const VoteHandle) -> bool {
    (*handle).read().unwrap().validate().is_err()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_rust_data_pointer(handle: *const VoteHandle) -> *const c_void {
    Arc::as_ptr(&(*handle)) as *const c_void
}
