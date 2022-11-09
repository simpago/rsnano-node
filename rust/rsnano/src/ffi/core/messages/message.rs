use super::MessageHeaderHandle;
use crate::{
    config::NetworkConstants,
    core::messages::{Message, MessageHeader},
    ffi::NetworkConstantsDto,
};

use std::ops::Deref;

pub struct MessageHandle(Box<dyn Message>);

impl MessageHandle {
    pub fn new(msg: Box<dyn Message>) -> *mut Self {
        Box::into_raw(Box::new(Self(msg)))
    }

    pub fn from_message<T: 'static + Message>(msg: T) -> *mut Self {
        Box::into_raw(Box::new(Self(Box::new(msg))))
    }
}

impl Deref for MessageHandle {
    type Target = Box<dyn Message>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header(
    handle: *mut MessageHandle,
) -> *mut MessageHeaderHandle {
    Box::into_raw(Box::new(MessageHeaderHandle::new(
        (*handle).0.header().clone(),
    )))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_set_header(
    handle: *mut MessageHandle,
    header: *mut MessageHeaderHandle,
) {
    (*handle).0.set_header((*header).deref())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_destroy(handle: *mut MessageHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_type(handle: *mut MessageHandle) -> u8 {
    (*handle).message_type() as u8
}

pub(crate) unsafe fn create_message_handle<T: 'static + Message>(
    constants: *mut NetworkConstantsDto,
    f: impl FnOnce(&NetworkConstants) -> T,
) -> *mut MessageHandle {
    let constants = NetworkConstants::try_from(&*constants).unwrap();
    Box::into_raw(Box::new(MessageHandle(Box::new(f(&constants)))))
}

pub(crate) unsafe fn create_message_handle2<T: 'static + Message>(
    header: *mut MessageHeaderHandle,
    f: impl FnOnce(MessageHeader) -> T,
) -> *mut MessageHandle {
    let msg = f((*header).deref().clone());
    Box::into_raw(Box::new(MessageHandle(Box::new(msg))))
}

pub(crate) unsafe fn message_handle_clone<T: 'static + Message + Clone>(
    handle: *mut MessageHandle,
) -> *mut MessageHandle {
    let msg = downcast_message::<T>(handle);
    Box::into_raw(Box::new(MessageHandle(Box::new(msg.clone()))))
}

pub(crate) unsafe fn downcast_message<T: 'static + Message>(
    handle: *mut MessageHandle,
) -> &'static T {
    (*handle).0.as_any().downcast_ref::<T>().unwrap()
}

pub(crate) unsafe fn downcast_message_mut<T: 'static + Message>(
    handle: *mut MessageHandle,
) -> &'static mut T {
    (*handle).0.as_any_mut().downcast_mut::<T>().unwrap()
}
