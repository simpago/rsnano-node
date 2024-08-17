use crate::NetworkConstantsDto;
use rsnano_messages::{DeserializedMessage, Message};
use rsnano_node::config::NetworkConstants;
use std::ops::{Deref, DerefMut};

pub struct MessageHandle(pub DeserializedMessage);

impl MessageHandle {
    pub fn new(msg: DeserializedMessage) -> *mut Self {
        Box::into_raw(Box::new(Self(msg)))
    }
}

impl Deref for MessageHandle {
    type Target = DeserializedMessage;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MessageHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_destroy(handle: *mut MessageHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_type(handle: *mut MessageHandle) -> u8 {
    (*handle).message.message_type() as u8
}

pub(crate) unsafe fn create_message_handle2(
    constants: *mut NetworkConstantsDto,
    f: impl FnOnce() -> Message,
) -> *mut MessageHandle {
    let constants = NetworkConstants::try_from(&*constants).unwrap();
    let msg = DeserializedMessage::new(f(), constants.protocol_info());
    MessageHandle::new(msg)
}

pub(crate) fn message_handle_clone(handle: &MessageHandle) -> *mut MessageHandle {
    MessageHandle::new(handle.deref().clone())
}
