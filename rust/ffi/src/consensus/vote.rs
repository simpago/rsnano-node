use crate::{utils::FfiStream, StringDto};
use rsnano_core::{
    utils::Serialize, BlockHash, FullHash, KeyPair, PublicKey, RawKey, Signature, Vote,
};
use std::{ffi::c_void, ops::Deref, sync::Arc};

pub struct VoteHandle(pub Arc<Vote>);

impl VoteHandle {
    pub fn new(vote: Arc<Vote>) -> *mut Self {
        Box::into_raw(Box::new(Self(vote)))
    }

    fn get_mut(&mut self) -> &mut Vote {
        Arc::get_mut(&mut self.0).expect("Could not make vote mutable")
    }
}

impl Deref for VoteHandle {
    type Target = Arc<Vote>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub extern "C" fn rsn_vote_create() -> *mut VoteHandle {
    VoteHandle::new(Arc::new(Vote::null()))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_create2(
    _account: *const u8,
    prv_key: *const u8,
    timestamp: u64,
    duration: u8,
    hashes: *const [u8; 32],
    hash_count: usize,
) -> *mut VoteHandle {
    let key = RawKey::from_ptr(prv_key);

    let hashes = if hashes.is_null() {
        &[]
    } else {
        std::slice::from_raw_parts(hashes, hash_count)
    };
    let hashes = hashes.iter().map(|&h| BlockHash::from_bytes(h)).collect();

    let keys = KeyPair::from_priv_key_bytes(key.as_bytes()).unwrap();
    VoteHandle::new(Arc::new(Vote::new(&keys, timestamp, duration, hashes)))
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

    VoteHandle::new(Arc::new((*handle).deref().deref().clone()))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_account(handle: &VoteHandle, result: *mut u8) {
    let result = std::slice::from_raw_parts_mut(result, 32);
    result.copy_from_slice(handle.voting_account.as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_account_set(handle: &mut VoteHandle, account: *const u8) {
    let mut bytes = [0; 32];
    bytes.copy_from_slice(std::slice::from_raw_parts(account, 32));
    handle.get_mut().voting_account = PublicKey::from_bytes(bytes);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_signature(handle: &VoteHandle, result: *mut u8) {
    let result = std::slice::from_raw_parts_mut(result, 64);
    result.copy_from_slice(handle.signature.as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_signature_set(handle: &mut VoteHandle, signature: *const u8) {
    let mut bytes = [0; 64];
    bytes.copy_from_slice(std::slice::from_raw_parts(signature, 64));
    handle.get_mut().signature = Signature::from_bytes(bytes);
}

#[repr(C)]
pub struct VoteHashesDto {
    pub handle: *mut VoteHashesHandle,
    pub count: usize,
    pub hashes: *const [u8; 32],
}

pub struct VoteHashesHandle {
    _data: Vec<[u8; 32]>,
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_hashes(handle: &VoteHandle) -> VoteHashesDto {
    let hashes: Vec<_> = handle.hashes.iter().map(|i| *i.as_bytes()).collect();
    let hashes_ptr = hashes.as_ptr();
    let count = hashes.len();
    let handle = Box::into_raw(Box::new(VoteHashesHandle { _data: hashes }));
    VoteHashesDto {
        handle,
        count,
        hashes: hashes_ptr,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_timestamp(handle: &VoteHandle) -> u64 {
    handle.timestamp()
}

#[no_mangle]
pub extern "C" fn rsn_vote_duration_bits(handle: &VoteHandle) -> u8 {
    handle.duration_bits()
}

#[no_mangle]
pub extern "C" fn rsn_vote_duration_ms(handle: &VoteHandle) -> u64 {
    handle.duration().as_millis() as u64
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

    (*first).eq(&(*second))
}

#[no_mangle]
pub extern "C" fn rsn_vote_hashes_string(handle: &VoteHandle) -> StringDto {
    handle.vote_hashes_string().into()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_hash(handle: &VoteHandle, result: *mut u8) {
    let hash = handle.hash();
    let result = std::slice::from_raw_parts_mut(result, 32);
    result.copy_from_slice(hash.as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_full_hash(handle: &VoteHandle, result: *mut u8) {
    let hash = handle.full_hash();
    let result = std::slice::from_raw_parts_mut(result, 32);
    result.copy_from_slice(hash.as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_serialize(handle: &VoteHandle, stream: *mut c_void) -> i32 {
    let mut stream = FfiStream::new(stream);
    handle.serialize(&mut stream);
    0
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_deserialize(handle: &mut VoteHandle, stream: *mut c_void) -> i32 {
    let mut stream = FfiStream::new(stream);
    match handle.get_mut().deserialize(&mut stream) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_validate(handle: &VoteHandle) -> bool {
    handle.validate().is_err()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_rust_data_pointer(handle: *const VoteHandle) -> *const c_void {
    Arc::as_ptr(&(*handle)) as *const c_void
}
