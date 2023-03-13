use std::ffi::c_void;

use rsnano_core::{Account, Signature};

use crate::{
    copy_account_bytes, copy_signature_bytes, utils::FfiStream, NetworkConstantsDto, StringDto,
};
use rsnano_node::messages::{
    Message, NodeIdHandshake, NodeIdHandshakeQuery, NodeIdHandshakeResponse,
};

use super::{
    create_message_handle, create_message_handle2, downcast_message, downcast_message_mut,
    message_handle_clone, MessageHandle, MessageHeaderHandle,
};

#[no_mangle]
pub unsafe extern "C" fn rsn_message_node_id_handshake_create(
    constants: *mut NetworkConstantsDto,
    query: *const u8,
    resp_account: *const u8,
    resp_signature: *const u8,
) -> *mut MessageHandle {
    let query = if !query.is_null() {
        let cookie = std::slice::from_raw_parts(query, 32).try_into().unwrap();
        Some(NodeIdHandshakeQuery { cookie })
    } else {
        None
    };

    let response = if !resp_account.is_null() && !resp_signature.is_null() {
        let node_id = Account::from_ptr(resp_account);
        let signature = Signature::from_ptr(resp_signature);
        Some(NodeIdHandshakeResponse { node_id, signature })
    } else {
        None
    };
    create_message_handle(constants, move |consts| {
        NodeIdHandshake::new(consts, query, response)
    })
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_node_id_handshake_create2(
    header: *mut MessageHeaderHandle,
) -> *mut MessageHandle {
    create_message_handle2(header, NodeIdHandshake::with_header)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_node_id_handshake_clone(
    handle: *mut MessageHandle,
) -> *mut MessageHandle {
    message_handle_clone::<NodeIdHandshake>(handle)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_node_id_handshake_query(
    handle: *mut MessageHandle,
    result: *mut u8,
) -> bool {
    match &downcast_message::<NodeIdHandshake>(handle).query {
        Some(query) => {
            std::slice::from_raw_parts_mut(result, 32).copy_from_slice(&query.cookie);
            true
        }
        None => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_node_id_handshake_response(
    handle: *mut MessageHandle,
    account: *mut u8,
    signature: *mut u8,
) -> bool {
    match &downcast_message::<NodeIdHandshake>(handle).response {
        Some(response) => {
            copy_account_bytes(response.node_id, account);
            copy_signature_bytes(&response.signature, signature);
            true
        }
        None => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_node_id_handshake_deserialize(
    handle: *mut MessageHandle,
    stream: *mut c_void,
) -> bool {
    let mut stream = FfiStream::new(stream);
    downcast_message_mut::<NodeIdHandshake>(handle)
        .deserialize(&mut stream)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_node_id_handshake_size(
    header: *mut MessageHeaderHandle,
) -> usize {
    NodeIdHandshake::serialized_size(&*header)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_node_id_handshake_serialize(
    handle: *mut MessageHandle,
    stream: *mut c_void,
) -> bool {
    let mut stream = FfiStream::new(stream);
    downcast_message::<NodeIdHandshake>(handle)
        .serialize(&mut stream)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_node_id_handshake_to_string(
    handle: *mut MessageHandle,
    result: *mut StringDto,
) {
    (*result) = downcast_message_mut::<NodeIdHandshake>(handle)
        .to_string()
        .into();
}
