use crate::transport::TcpMessageManager;
use std::{ops::Deref, sync::Arc};

use super::TcpMessageItemHandle;

pub struct TcpMessageManagerHandle(Arc<TcpMessageManager>);

impl Deref for TcpMessageManagerHandle {
    type Target = Arc<TcpMessageManager>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub extern "C" fn rsn_tcp_message_manager_create(
    incoming_connections_max: usize,
) -> *mut TcpMessageManagerHandle {
    Box::into_raw(Box::new(TcpMessageManagerHandle(Arc::new(
        TcpMessageManager::new(incoming_connections_max),
    ))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_message_manager_destroy(handle: *mut TcpMessageManagerHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_message_manager_put_message(
    handle: *mut TcpMessageManagerHandle,
    msg: *const TcpMessageItemHandle,
) {
    (*handle).0.put_message((*msg).deref().clone())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_message_manager_get_message(
    handle: *mut TcpMessageManagerHandle,
) -> *mut TcpMessageItemHandle {
    let msg = (*handle).0.get_message();
    TcpMessageItemHandle::new(msg)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_message_manager_stop(handle: *mut TcpMessageManagerHandle) {
    (*handle).0.stop();
}
