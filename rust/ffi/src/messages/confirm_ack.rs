use std::{
    ffi::c_void,
    ops::Deref,
    sync::{Arc, RwLock},
};

use crate::{
    utils::FfiStream,
    voting::{VoteHandle, VoteUniquerHandle},
    NetworkConstantsDto, StringDto,
};
use rsnano_node::{
    config::NetworkConstants,
    messages::{ConfirmAck, Message},
    voting::Vote,
};

use super::{
    create_message_handle, create_message_handle2, downcast_message, message_handle_clone,
    MessageHandle, MessageHeaderHandle,
};

#[no_mangle]
pub unsafe extern "C" fn rsn_message_confirm_ack_create(
    constants: *mut NetworkConstantsDto,
    vote: *mut VoteHandle,
) -> *mut MessageHandle {
    create_message_handle(constants, |consts| {
        let vote = (*vote).clone();
        ConfirmAck::new(consts, vote)
    })
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_confirm_ack_create2(
    header: *mut MessageHeaderHandle,
    stream: *mut c_void,
    uniquer: *mut VoteUniquerHandle,
    is_error: *mut bool,
) -> *mut MessageHandle {
    create_message_handle2(header, |hdr| {
        let mut stream = FfiStream::new(stream);
        let uniquer = if uniquer.is_null() {
            None
        } else {
            Some((*uniquer).deref().as_ref())
        };

        match ConfirmAck::with_header(hdr, &mut stream, uniquer) {
            Ok(i) => i,
            Err(_) => {
                *is_error = true;
                //workaround to prevent nullptr:
                ConfirmAck::new(
                    &NetworkConstants::empty(),
                    Arc::new(RwLock::new(Vote::null())),
                )
            }
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_confirm_ack_clone(
    handle: *mut MessageHandle,
) -> *mut MessageHandle {
    message_handle_clone::<ConfirmAck>(handle)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_confirm_ack_vote(
    handle: *mut MessageHandle,
) -> *mut VoteHandle {
    match downcast_message::<ConfirmAck>(handle).vote() {
        Some(vote) => Box::into_raw(Box::new(VoteHandle::new(vote.clone()))),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_confirm_ack_size(count: usize) -> usize {
    ConfirmAck::serialized_size(count as u8)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_confirm_ack_serialize(
    handle: *mut MessageHandle,
    stream: *mut c_void,
) -> bool {
    let mut stream = FfiStream::new(stream);
    downcast_message::<ConfirmAck>(handle)
        .serialize(&mut stream)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_confirm_ack_to_string(
    handle: *mut MessageHandle,
    result: *mut StringDto,
) {
    (*result) = downcast_message::<ConfirmAck>(handle).to_string().into();
}
