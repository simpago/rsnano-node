use std::ffi::c_void;
use std::sync::{Arc, RwLock};

use crate::core::{
    BlockEnum, BlockHash, LazyBlockHash, PublicKey, RawKey, ReceiveBlock, ReceiveHashables,
    Signature,
};
use crate::ffi::utils::FfiStream;
use crate::ffi::FfiPropertyTreeReader;

use super::BlockHandle;

#[repr(C)]
pub struct ReceiveBlockDto {
    pub work: u64,
    pub signature: [u8; 64],
    pub previous: [u8; 32],
    pub source: [u8; 32],
}

#[repr(C)]
pub struct ReceiveBlockDto2 {
    pub previous: [u8; 32],
    pub source: [u8; 32],
    pub priv_key: [u8; 32],
    pub pub_key: [u8; 32],
    pub work: u64,
}

unsafe fn read_receive_block<T>(
    handle: *const BlockHandle,
    f: impl FnOnce(&ReceiveBlock) -> T,
) -> T {
    let block = (*handle).block.read().unwrap();
    match &*block {
        BlockEnum::Receive(b) => f(b),
        _ => panic!("expected receive block"),
    }
}

unsafe fn write_receive_block<T>(
    handle: *mut BlockHandle,
    mut f: impl FnMut(&mut ReceiveBlock) -> T,
) -> T {
    let mut block = (*handle).block.write().unwrap();
    match &mut *block {
        BlockEnum::Receive(b) => f(b),
        _ => panic!("expected receive block"),
    }
}

#[no_mangle]
pub extern "C" fn rsn_receive_block_create(dto: &ReceiveBlockDto) -> *mut BlockHandle {
    Box::into_raw(Box::new(BlockHandle {
        block: Arc::new(RwLock::new(BlockEnum::Receive(ReceiveBlock {
            work: dto.work,
            signature: Signature::from_bytes(dto.signature),
            hashables: ReceiveHashables {
                previous: BlockHash::from_bytes(dto.previous),
                source: BlockHash::from_bytes(dto.source),
            },
            hash: LazyBlockHash::new(),
            sideband: None,
        }))),
    }))
}

#[no_mangle]
pub extern "C" fn rsn_receive_block_create2(dto: &ReceiveBlockDto2) -> *mut BlockHandle {
    let block = match ReceiveBlock::new(
        BlockHash::from_bytes(dto.previous),
        BlockHash::from_bytes(dto.source),
        &RawKey::from_bytes(dto.priv_key),
        &PublicKey::from_bytes(dto.pub_key),
        dto.work,
    ) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("could not create receive block: {}", e);
            return std::ptr::null_mut();
        }
    };

    Box::into_raw(Box::new(BlockHandle {
        block: Arc::new(RwLock::new(BlockEnum::Receive(block))),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_receive_block_previous_set(
    handle: *mut BlockHandle,
    previous: &[u8; 32],
) {
    let previous = BlockHash::from_bytes(*previous);
    write_receive_block(handle, |b| b.hashables.previous = previous);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_receive_block_source(
    handle: *const BlockHandle,
    result: *mut [u8; 32],
) {
    (*result) = read_receive_block(handle, |b| *b.hashables.source.as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_receive_block_source_set(
    handle: *mut BlockHandle,
    previous: &[u8; 32],
) {
    let source = BlockHash::from_bytes(*previous);
    write_receive_block(handle, |b| b.hashables.source = source);
}

#[no_mangle]
pub extern "C" fn rsn_receive_block_size() -> usize {
    ReceiveBlock::serialized_size()
}

#[no_mangle]
pub extern "C" fn rsn_receive_block_deserialize_json(ptree: *const c_void) -> *mut BlockHandle {
    let reader = FfiPropertyTreeReader::new(ptree);
    match ReceiveBlock::deserialize_json(&reader) {
        Ok(block) => Box::into_raw(Box::new(BlockHandle {
            block: Arc::new(RwLock::new(BlockEnum::Receive(block))),
        })),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_receive_block_deserialize(stream: *mut c_void) -> *mut BlockHandle {
    let mut stream = FfiStream::new(stream);
    match ReceiveBlock::deserialize(&mut stream) {
        Ok(block) => Box::into_raw(Box::new(BlockHandle {
            block: Arc::new(RwLock::new(BlockEnum::Receive(block))),
        })),
        Err(_) => std::ptr::null_mut(),
    }
}
