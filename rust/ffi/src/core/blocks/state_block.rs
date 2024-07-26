use std::ffi::c_void;
use std::ops::Deref;
use std::sync::Arc;

use crate::{utils::FfiStream, FfiPropertyTree};
use rsnano_core::{
    Account, Amount, BlockEnum, BlockHash, LazyBlockHash, Link, PublicKey, RawKey, Signature,
    StateBlock, StateHashables,
};

use super::BlockHandle;

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

unsafe fn read_state_block<T>(handle: *const BlockHandle, f: impl FnOnce(&StateBlock) -> T) -> T {
    let block = (*handle).deref();
    match block.deref() {
        BlockEnum::State(b) => f(b),
        _ => panic!("expected state block"),
    }
}

unsafe fn write_state_block<T>(
    handle: *mut BlockHandle,
    mut f: impl FnMut(&mut StateBlock) -> T,
) -> T {
    let block = (*handle).get_mut();
    match block {
        BlockEnum::State(b) => f(b),
        _ => panic!("expected state block"),
    }
}

#[no_mangle]
pub extern "C" fn rsn_state_block_create(dto: &StateBlockDto) -> *mut BlockHandle {
    Box::into_raw(Box::new(BlockHandle(Arc::new(BlockEnum::State(
        StateBlock {
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
    )))))
}

#[no_mangle]
pub extern "C" fn rsn_state_block_create2(dto: &StateBlockDto2) -> *mut BlockHandle {
    let block = StateBlock::new_obsolete(
        Account::from_bytes(dto.account),
        BlockHash::from_bytes(dto.previous),
        Account::from_bytes(dto.representative),
        Amount::from_be_bytes(dto.balance),
        Link::from_bytes(dto.link),
        &RawKey::from_bytes(dto.priv_key),
        &PublicKey::from_bytes(dto.pub_key),
        dto.work,
    );
    Box::into_raw(Box::new(BlockHandle(Arc::new(BlockEnum::State(block)))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_account(
    handle: *const BlockHandle,
    result: *mut [u8; 32],
) {
    (*result) = read_state_block(handle, |b| *b.hashables.account.as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_account_set(handle: *mut BlockHandle, source: &[u8; 32]) {
    write_state_block(handle, |b| {
        b.hashables.account = Account::from_bytes(*source);
        b.hash.clear();
    });
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_previous_set(handle: *mut BlockHandle, source: &[u8; 32]) {
    write_state_block(handle, |b| {
        b.hashables.previous = BlockHash::from_bytes(*source);
        b.hash.clear();
    });
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_representative(
    handle: *const BlockHandle,
    result: *mut [u8; 32],
) {
    (*result) = read_state_block(handle, |b| *b.hashables.representative.as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_representative_set(
    handle: *mut BlockHandle,
    representative: &[u8; 32],
) {
    write_state_block(handle, |b| {
        b.hashables.representative = Account::from_bytes(*representative);
        b.hash.clear();
    })
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_balance(
    handle: *const BlockHandle,
    result: *mut [u8; 16],
) {
    (*result) = read_state_block(handle, |b| b.hashables.balance.to_be_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_balance_set(handle: *mut BlockHandle, balance: &[u8; 16]) {
    write_state_block(handle, |b| {
        b.hashables.balance = Amount::from_be_bytes(*balance);
        b.hash.clear();
    });
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_link(handle: *const BlockHandle, result: *mut [u8; 32]) {
    (*result) = read_state_block(handle, |b| *b.hashables.link.as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_link_set(handle: *mut BlockHandle, link: &[u8; 32]) {
    write_state_block(handle, |b| {
        b.hashables.link = Link::from_bytes(*link);
        b.hash.clear();
    })
}

#[no_mangle]
pub extern "C" fn rsn_state_block_size() -> usize {
    StateBlock::serialized_size()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_state_block_deserialize(stream: *mut c_void) -> *mut BlockHandle {
    let mut stream = FfiStream::new(stream);
    match StateBlock::deserialize(&mut stream) {
        Ok(block) => Box::into_raw(Box::new(BlockHandle(Arc::new(BlockEnum::State(block))))),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn rsn_state_block_deserialize_json(ptree: *mut c_void) -> *mut BlockHandle {
    let reader = FfiPropertyTree::new_borrowed(ptree);
    match StateBlock::deserialize_json(&reader) {
        Ok(block) => Box::into_raw(Box::new(BlockHandle(Arc::new(BlockEnum::State(block))))),
        Err(_) => std::ptr::null_mut(),
    }
}
