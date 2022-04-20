use std::ffi::c_void;

use crate::{
    Account, Amount, Block, BlockHash, LazyBlockHash, Link, PublicKey, RawKey, Signature,
    StateBlock, StateHashables,
};

use crate::ffi::{FfiPropertyTreeReader, FfiPropertyTreeWriter, FfiStream};

pub struct StateBlockHandle {
    pub block: StateBlock,
}

#[repr(C)]
pub struct StateBlockDto {
    pub signature: [u8; 64],
    pub account: [u8; 32],
    pub previous: [u8; 32],
    pub representative: [u8; 32],
    pub link: [u8; 32],
    pub balance: [u8; 16],
    pub work: u64,
}

#[repr(C)]
pub struct StateBlockDto2 {
    pub account: [u8; 32],
    pub previous: [u8; 32],
    pub representative: [u8; 32],
    pub link: [u8; 32],
    pub balance: [u8; 16],
    pub priv_key: [u8; 32],
    pub pub_key: [u8; 32],
    pub work: u64,
}

#[no_mangle]
pub extern "C" fn rsn_state_block_create(dto: &StateBlockDto) -> *mut StateBlockHandle {
    Box::into_raw(Box::new(StateBlockHandle {
        block: StateBlock {
            work: dto.work,
            signature: Signature::from_bytes(dto.signature),
            hashables: StateHashables {
                account: Account::from_bytes(dto.account),
                previous: BlockHash::from_bytes(dto.previous),
                representative: Account::from_bytes(dto.representative),
                balance: Amount::from_be_bytes(dto.balance),
                link: Link::from_bytes(dto.link),
            },
            hash: LazyBlockHash::new(),
            sideband: None,
        },
    }))
}

#[no_mangle]
pub extern "C" fn rsn_state_block_create2(dto: &StateBlockDto2) -> *mut StateBlockHandle {
    let block = match StateBlock::new(
        Account::from_bytes(dto.account),
        BlockHash::from_bytes(dto.previous),
        Account::from_bytes(dto.representative),
        Amount::from_be_bytes(dto.balance),
        Link::from_bytes(dto.link),
        &RawKey::from_bytes(dto.priv_key),
        &PublicKey::from_bytes(dto.pub_key),
        dto.work,
    ) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("could not create state block: {}", e);
            return std::ptr::null_mut();
        }
    };

    Box::into_raw(Box::new(StateBlockHandle { block }))
}

#[no_mangle]
pub extern "C" fn rsn_state_block_clone(handle: &StateBlockHandle) -> *mut StateBlockHandle {
    Box::into_raw(Box::new(StateBlockHandle {
        block: handle.block.clone(),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_destroy(handle: *mut StateBlockHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_work_set(handle: *mut StateBlockHandle, work: u64) {
    (*handle).block.work = work;
}

#[no_mangle]
pub extern "C" fn rsn_state_block_work(handle: &StateBlockHandle) -> u64 {
    handle.block.work
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_signature(
    handle: &StateBlockHandle,
    result: *mut [u8; 64],
) {
    (*result) = (*handle).block.signature.to_be_bytes();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_signature_set(
    handle: *mut StateBlockHandle,
    signature: &[u8; 64],
) {
    (*handle).block.signature = Signature::from_bytes(*signature);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_account(handle: &StateBlockHandle, result: *mut [u8; 32]) {
    (*result) = (*handle).block.hashables.account.to_bytes();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_account_set(
    handle: *mut StateBlockHandle,
    source: &[u8; 32],
) {
    (*handle).block.hashables.account = Account::from_bytes(*source);
    (*handle).block.hash.clear();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_previous(
    handle: &StateBlockHandle,
    result: *mut [u8; 32],
) {
    (*result) = (*handle).block.hashables.previous.to_bytes();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_previous_set(
    handle: *mut StateBlockHandle,
    source: &[u8; 32],
) {
    (*handle).block.hashables.previous = BlockHash::from_bytes(*source);
    (*handle).block.hash.clear();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_representative(
    handle: &StateBlockHandle,
    result: *mut [u8; 32],
) {
    (*result) = (*handle).block.hashables.representative.to_bytes();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_representative_set(
    handle: *mut StateBlockHandle,
    representative: &[u8; 32],
) {
    (*handle).block.hashables.representative = Account::from_bytes(*representative);
    (*handle).block.hash.clear();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_balance(handle: &StateBlockHandle, result: *mut [u8; 16]) {
    (*result) = (*handle).block.hashables.balance.to_be_bytes();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_balance_set(
    handle: *mut StateBlockHandle,
    balance: &[u8; 16],
) {
    (*handle).block.hashables.balance = Amount::from_be_bytes(*balance);
    (*handle).block.hash.clear();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_link(handle: &StateBlockHandle, result: *mut [u8; 32]) {
    (*result) = (*handle).block.hashables.link.to_bytes();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_link_set(handle: *mut StateBlockHandle, link: &[u8; 32]) {
    (*handle).block.hashables.link = Link::from_bytes(*link);
    (*handle).block.hash.clear();
}

#[no_mangle]
pub extern "C" fn rsn_state_block_equals(a: &StateBlockHandle, b: &StateBlockHandle) -> bool {
    a.block.work.eq(&b.block.work)
        && a.block.signature.eq(&b.block.signature)
        && a.block.hashables.eq(&b.block.hashables)
}

#[no_mangle]
pub extern "C" fn rsn_state_block_size() -> usize {
    StateBlock::serialized_size()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_hash(handle: &StateBlockHandle, hash: *mut [u8; 32]) {
    (*hash) = handle.block.hash().to_bytes();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_serialize(
    handle: *mut StateBlockHandle,
    stream: *mut c_void,
) -> i32 {
    let mut stream = FfiStream::new(stream);
    if (*handle).block.serialize(&mut stream).is_ok() {
        0
    } else {
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_deserialize(stream: *mut c_void) -> *mut StateBlockHandle {
    let mut stream = FfiStream::new(stream);
    match StateBlock::deserialize(&mut stream) {
        Ok(block) => Box::into_raw(Box::new(StateBlockHandle { block })),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn rsn_state_block_serialize_json(
    handle: &StateBlockHandle,
    ptree: *mut c_void,
) -> i32 {
    let mut writer = FfiPropertyTreeWriter::new(ptree);
    match handle.block.serialize_json(&mut writer) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn rsn_state_block_deserialize_json(ptree: *const c_void) -> *mut StateBlockHandle {
    let reader = FfiPropertyTreeReader::new(ptree);
    match StateBlock::deserialize_json(&reader) {
        Ok(block) => Box::into_raw(Box::new(StateBlockHandle { block })),
        Err(_) => std::ptr::null_mut(),
    }
}
