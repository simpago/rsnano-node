use std::{ffi::c_void, net::SocketAddr};

use crate::{
    ffi::{network::EndpointDto, FfiStream, NetworkConstantsDto},
    messages::{Keepalive, Message},
};

use super::{
    create_message_handle, create_message_handle2, downcast_message, downcast_message_mut,
    message_handle_clone, MessageHandle, MessageHeaderHandle,
};

#[no_mangle]
pub unsafe extern "C" fn rsn_message_keepalive_create(
    constants: *mut NetworkConstantsDto,
    version_using: i16,
) -> *mut MessageHandle {
    create_message_handle(constants, |consts| {
        if version_using < 0 {
            Keepalive::new(consts)
        } else {
            Keepalive::with_version_using(consts, version_using as u8)
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_keepalive_create2(
    header: *mut MessageHeaderHandle,
) -> *mut MessageHandle {
    create_message_handle2(header, Keepalive::with_header)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_keepalive_clone(
    handle: *mut MessageHandle,
) -> *mut MessageHandle {
    message_handle_clone::<Keepalive>(handle)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_keepalive_peers(
    handle: *mut MessageHandle,
    result: *mut EndpointDto,
) {
    let dtos = std::slice::from_raw_parts_mut(result, 8);
    let peers: Vec<_> = downcast_message::<Keepalive>(handle)
        .peers()
        .iter()
        .map(EndpointDto::from)
        .collect();
    dtos.clone_from_slice(&peers);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_keepalive_set_peers(
    handle: *mut MessageHandle,
    result: *const EndpointDto,
) {
    let dtos = std::slice::from_raw_parts(result, 8);
    let peers: [SocketAddr; 8] = dtos
        .iter()
        .map(SocketAddr::from)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    downcast_message_mut::<Keepalive>(handle).set_peers(&peers);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_keepalive_serialize(
    handle: *mut MessageHandle,
    stream: *mut c_void,
) -> bool {
    let mut stream = FfiStream::new(stream);
    downcast_message::<Keepalive>(handle)
        .serialize(&mut stream)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_keepalive_deserialize(
    handle: *mut MessageHandle,
    stream: *mut c_void,
) -> bool {
    let mut stream = FfiStream::new(stream);
    downcast_message_mut::<Keepalive>(handle)
        .deserialize(&mut stream)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_keepalive_size() -> usize {
    Keepalive::serialized_size()
}
