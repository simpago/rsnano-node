mod block_details;
mod block_uniquer;
mod change_block;
mod open_block;
mod receive_block;
mod send_block;
mod state_block;

use std::{
    convert::TryFrom,
    ffi::c_void,
    ops::Deref,
    sync::{Arc, RwLock},
};

pub use block_details::*;
pub use block_uniquer::BlockUniquerHandle;
pub use change_block::*;
pub use open_block::*;
pub use receive_block::*;
pub use send_block::*;
pub use state_block::*;

use crate::{
    core::{
        deserialize_block, deserialize_block_json, serialized_block_size, BlockEnum, BlockSideband,
        BlockType, Signature,
    },
    ffi::{utils::FfiStream, FfiPropertyTreeReader, FfiPropertyTreeWriter},
};
use num::FromPrimitive;

#[no_mangle]
pub extern "C" fn rsn_block_serialized_size(block_type: u8) -> usize {
    match FromPrimitive::from_u8(block_type) {
        Some(block_type) => serialized_block_size(block_type),
        None => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_sideband(
    block: *const BlockHandle,
    sideband: *mut BlockSidebandDto,
) -> i32 {
    let b = (*block).block.read().unwrap();
    let block = (&*b).as_block();
    match block.sideband() {
        Some(sb) => {
            set_block_sideband_dto(sb, sideband);
            0
        }
        None => -1,
    }
}

#[no_mangle]
pub extern "C" fn rsn_block_clone(handle: &BlockHandle) -> *mut BlockHandle {
    let cloned = handle.block.read().unwrap().clone();
    Box::into_raw(Box::new(BlockHandle {
        block: Arc::new(RwLock::new(cloned)),
    }))
}

#[no_mangle]
pub extern "C" fn rsn_block_handle_clone(handle: &BlockHandle) -> *mut BlockHandle {
    Box::into_raw(Box::new(BlockHandle {
        block: handle.block.clone(),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_rust_data_pointer(handle: *const BlockHandle) -> *const c_void {
    let ptr = Arc::as_ptr(&(*handle).block);
    ptr as *const c_void
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_destroy(handle: *mut BlockHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_sideband_set(
    block: *mut BlockHandle,
    sideband: &BlockSidebandDto,
) -> i32 {
    match BlockSideband::try_from(sideband) {
        Ok(sideband) => {
            (*block)
                .block
                .write()
                .unwrap()
                .as_block_mut()
                .set_sideband(sideband);
            0
        }
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_has_sideband(block: *const BlockHandle) -> bool {
    let block = (*block).block.read().unwrap();
    block.as_block().sideband().is_some()
}

pub struct BlockHandle {
    pub(crate) block: Arc<RwLock<BlockEnum>>,
}

impl BlockHandle {
    pub fn new(block: Arc<RwLock<BlockEnum>>) -> Self {
        Self { block }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_deserialize_block_json(ptree: *const c_void) -> *mut BlockHandle {
    let ptree_reader = FfiPropertyTreeReader::new(ptree);
    match deserialize_block_json(&ptree_reader) {
        Ok(block) => Box::into_raw(Box::new(BlockHandle {
            block: Arc::new(RwLock::new(block)),
        })),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_type(handle: *const BlockHandle) -> u8 {
    (*handle).block.read().unwrap().as_block().block_type() as u8
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_work_set(handle: *mut BlockHandle, work: u64) {
    (*handle)
        .block
        .write()
        .unwrap()
        .as_block_mut()
        .set_work(work);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_work(handle: *const BlockHandle) -> u64 {
    (*handle).block.read().unwrap().as_block().work()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_signature(handle: *const BlockHandle, result: *mut [u8; 64]) {
    (*result) = *(*handle)
        .block
        .read()
        .unwrap()
        .as_block()
        .block_signature()
        .as_bytes();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_signature_set(handle: *mut BlockHandle, signature: &[u8; 64]) {
    (*handle)
        .block
        .write()
        .unwrap()
        .as_block_mut()
        .set_block_signature(&Signature::from_bytes(*signature));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_previous(handle: &BlockHandle, result: *mut [u8; 32]) {
    (*result) = *(*handle)
        .block
        .read()
        .unwrap()
        .as_block()
        .previous()
        .as_bytes();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_equals(a: *const BlockHandle, b: *const BlockHandle) -> bool {
    let a_guard = (*a).block.read().unwrap();
    let b_guard = (*b).block.read().unwrap();

    (*a_guard).eq(&*b_guard)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_hash(handle: *const BlockHandle, hash: *mut [u8; 32]) {
    (*hash) = *(*handle).block.read().unwrap().as_block().hash().as_bytes();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_full_hash(handle: *const BlockHandle, hash: *mut u8) {
    let result = std::slice::from_raw_parts_mut(hash, 32);
    let hash = (*handle).block.read().unwrap().as_block().full_hash();

    result.copy_from_slice(hash.as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_serialize(handle: *mut BlockHandle, stream: *mut c_void) -> i32 {
    let mut stream = FfiStream::new(stream);
    if (*handle)
        .block
        .read()
        .unwrap()
        .as_block()
        .serialize(&mut stream)
        .is_ok()
    {
        0
    } else {
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_serialize_json(
    handle: *const BlockHandle,
    ptree: *mut c_void,
) -> i32 {
    let mut writer = FfiPropertyTreeWriter::new_borrowed(ptree);
    match (*handle)
        .block
        .read()
        .unwrap()
        .as_block()
        .serialize_json(&mut writer)
    {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_deserialize_block(
    block_type: u8,
    stream: *mut c_void,
    uniquer: *mut BlockUniquerHandle,
) -> *mut BlockHandle {
    let mut stream = FfiStream::new(stream);
    let block_type = match BlockType::from_u8(block_type) {
        Some(bt) => bt,
        None => return std::ptr::null_mut(),
    };

    let uniquer = if uniquer.is_null() {
        None
    } else {
        Some((*uniquer).deref().as_ref())
    };

    match deserialize_block(block_type, &mut stream, uniquer) {
        Ok(block) => Box::into_raw(Box::new(BlockHandle::new(block))),
        Err(_) => std::ptr::null_mut(),
    }
}

pub struct BlockArrayRawPtr(Vec<*mut BlockHandle>);

pub(crate) unsafe fn copy_block_array_dto(
    blocks: Vec<Arc<RwLock<BlockEnum>>>,
    target: *mut BlockArrayDto,
) {
    let mut raw_block_array = Box::new(BlockArrayRawPtr(Vec::new()));
    for block in blocks {
        raw_block_array
            .0
            .push(Box::into_raw(Box::new(BlockHandle::new(block))));
    }
    (*target).blocks = raw_block_array.0.as_ptr();
    (*target).count = raw_block_array.0.len();
    (*target).raw_ptr = Box::into_raw(raw_block_array);
}

#[repr(C)]
pub struct BlockArrayDto {
    pub blocks: *const *mut BlockHandle,
    pub count: usize,
    pub raw_ptr: *mut BlockArrayRawPtr,
}

impl BlockArrayDto {
    pub unsafe fn blocks(&self) -> impl Iterator<Item = &Arc<RwLock<BlockEnum>>> {
        let dtos = std::slice::from_raw_parts(self.blocks, self.count);
        dtos.iter().map(|&b| &(*b).block)
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_array_destroy(dto: *mut BlockArrayDto) {
    drop(Box::from_raw((*dto).raw_ptr))
}
