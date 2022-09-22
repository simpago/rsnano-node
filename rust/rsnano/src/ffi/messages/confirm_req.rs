use std::{ffi::c_void, ops::Deref, sync::Arc};

use crate::{
    ffi::{BlockHandle, BlockUniquerHandle, FfiStream, NetworkConstantsDto, StringDto},
    messages::{ConfirmReq, Message},
    BlockHash, BlockType, Root,
};

use super::{
    create_message_handle, create_message_handle2, downcast_message, downcast_message_mut,
    message_handle_clone, MessageHandle, MessageHeaderHandle,
};
use num_traits::FromPrimitive;

#[repr(C)]
pub struct HashRootPair {
    pub block_hash: [u8; 32],
    pub root: [u8; 32],
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_confirm_req_create(
    constants: *mut NetworkConstantsDto,
    block: *mut BlockHandle,
    roots_hashes: *const HashRootPair,
    roots_hashes_count: usize,
) -> *mut MessageHandle {
    create_message_handle(constants, |consts| {
        if !block.is_null() {
            let block = (*block).block.clone();
            ConfirmReq::with_block(consts, block)
        } else {
            let dtos = std::slice::from_raw_parts(roots_hashes, roots_hashes_count);
            let roots_hashes = dtos
                .iter()
                .map(|dto| {
                    (
                        BlockHash::from_bytes(dto.block_hash),
                        Root::from_bytes(dto.root),
                    )
                })
                .collect();
            ConfirmReq::with_roots_hashes(consts, roots_hashes)
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_confirm_req_create2(
    header: *mut MessageHeaderHandle,
) -> *mut MessageHandle {
    create_message_handle2(header, ConfirmReq::with_header)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_confirm_req_clone(
    handle: *mut MessageHandle,
) -> *mut MessageHandle {
    message_handle_clone::<ConfirmReq>(handle)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_confirm_req_block(
    handle: *mut MessageHandle,
) -> *mut BlockHandle {
    match downcast_message::<ConfirmReq>(handle).block() {
        Some(block) => Box::into_raw(Box::new(BlockHandle::new(Arc::clone(block)))),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_confirm_req_roots_hashes_count(
    handle: *mut MessageHandle,
) -> usize {
    downcast_message::<ConfirmReq>(handle).roots_hashes().len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_confirm_req_roots_hashes(
    handle: *mut MessageHandle,
    result: *mut HashRootPair,
) {
    let req = downcast_message::<ConfirmReq>(handle);
    let result_slice = std::slice::from_raw_parts_mut(result, req.roots_hashes().len());
    for (i, (hash, root)) in req.roots_hashes().iter().enumerate() {
        result_slice[i] = HashRootPair {
            block_hash: hash.to_bytes(),
            root: root.to_bytes(),
        };
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_confirm_req_serialize(
    handle: *mut MessageHandle,
    stream: *mut c_void,
) -> bool {
    let mut stream = FfiStream::new(stream);
    downcast_message_mut::<ConfirmReq>(handle)
        .serialize(&mut stream)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_confirm_req_deserialize(
    handle: *mut MessageHandle,
    stream: *mut c_void,
    uniquer: *mut BlockUniquerHandle,
) -> bool {
    let mut stream = FfiStream::new(stream);
    let uniquer = if uniquer.is_null() {
        None
    } else {
        Some((*uniquer).deref().as_ref())
    };
    downcast_message_mut::<ConfirmReq>(handle)
        .deserialize(&mut stream, uniquer)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_confirm_req_equals(
    handle_a: *mut MessageHandle,
    handle_b: *mut MessageHandle,
) -> bool {
    let a = downcast_message_mut::<ConfirmReq>(handle_a);
    let b = downcast_message_mut::<ConfirmReq>(handle_b);
    a == b
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_confirm_req_roots_string(
    handle: *mut MessageHandle,
    result: *mut StringDto,
) {
    let req = downcast_message_mut::<ConfirmReq>(handle);
    (*result) = req.roots_string().into();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_confirm_req_size(block_type: u8, count: usize) -> usize {
    ConfirmReq::serialized_size(BlockType::from_u8(block_type).unwrap(), count as u8)
}
