use super::BlockHandle;
use crate::utils::FfiStream;
use rsnano_core::{
    Account, BlockEnum, BlockHash, LazyBlockHash, OpenBlock, OpenHashables, PublicKey, RawKey,
    Signature,
};
use std::ffi::c_void;
use std::ops::Deref;
use std::sync::Arc;

#[repr(C)]
pub struct OpenBlockDto {
    pub work: u64,
    pub signature: [u8; 64],
    pub source: [u8; 32],
    pub representative: [u8; 32],
    pub account: [u8; 32],
}

#[repr(C)]
pub struct OpenBlockDto2 {
    pub source: [u8; 32],
    pub representative: [u8; 32],
    pub account: [u8; 32],
    pub priv_key: [u8; 32],
    pub pub_key: [u8; 32],
    pub work: u64,
}

#[no_mangle]
pub extern "C" fn rsn_open_block_create(dto: &OpenBlockDto) -> *mut BlockHandle {
    Box::into_raw(Box::new(BlockHandle(Arc::new(BlockEnum::LegacyOpen(
        OpenBlock {
            work: dto.work,
            signature: Signature::from_bytes(dto.signature),
            hashables: OpenHashables {
                source: BlockHash::from_bytes(dto.source),
                representative: PublicKey::from_bytes(dto.representative),
                account: Account::from_bytes(dto.account),
            },
            hash: LazyBlockHash::new(),
            sideband: None,
        },
    )))))
}

#[no_mangle]
pub extern "C" fn rsn_open_block_create2(dto: &OpenBlockDto2) -> *mut BlockHandle {
    let block = OpenBlock::new(
        BlockHash::from_bytes(dto.source),
        PublicKey::from_bytes(dto.representative),
        Account::from_bytes(dto.account),
        &RawKey::from_bytes(dto.priv_key),
        dto.work,
    );

    Box::into_raw(Box::new(BlockHandle(Arc::new(BlockEnum::LegacyOpen(
        block,
    )))))
}

unsafe fn read_open_block<T>(handle: *const BlockHandle, f: impl FnOnce(&OpenBlock) -> T) -> T {
    let block = (*handle).deref().deref();
    match block {
        BlockEnum::LegacyOpen(b) => f(b),
        _ => panic!("expected open block"),
    }
}

unsafe fn write_open_block<T>(
    handle: *mut BlockHandle,
    mut f: impl FnMut(&mut OpenBlock) -> T,
) -> T {
    let block = (*handle).get_mut();
    match block {
        BlockEnum::LegacyOpen(b) => f(b),
        _ => panic!("expected open block"),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_open_block_source(handle: *const BlockHandle, result: *mut [u8; 32]) {
    (*result) = read_open_block(handle, |b| *b.hashables.source.as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_open_block_source_set(handle: *mut BlockHandle, source: &[u8; 32]) {
    write_open_block(handle, |b| {
        b.hashables.source = BlockHash::from_bytes(*source)
    });
}

#[no_mangle]
pub unsafe extern "C" fn rsn_open_block_representative(
    handle: *const BlockHandle,
    result: *mut [u8; 32],
) {
    (*result) = read_open_block(handle, |b| *b.hashables.representative.as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_open_block_representative_set(
    handle: *mut BlockHandle,
    representative: &[u8; 32],
) {
    write_open_block(handle, |b| {
        b.hashables.representative = PublicKey::from_bytes(*representative)
    });
}

#[no_mangle]
pub unsafe extern "C" fn rsn_open_block_account(handle: *const BlockHandle, result: *mut [u8; 32]) {
    (*result) = read_open_block(handle, |b| *b.hashables.account.as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_open_block_account_set(handle: *mut BlockHandle, account: &[u8; 32]) {
    write_open_block(handle, |b| {
        b.hashables.account = Account::from_bytes(*account)
    });
}

#[no_mangle]
pub extern "C" fn rsn_open_block_size() -> usize {
    OpenBlock::serialized_size()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_open_block_deserialize(stream: *mut c_void) -> *mut BlockHandle {
    let mut stream = FfiStream::new(stream);
    match OpenBlock::deserialize(&mut stream) {
        Ok(block) => Box::into_raw(Box::new(BlockHandle(Arc::new(BlockEnum::LegacyOpen(
            block,
        ))))),
        Err(_) => std::ptr::null_mut(),
    }
}
