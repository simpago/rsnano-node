use std::{net::SocketAddr, ops::Deref};

use crate::{
    ffi::{copy_account_bytes, messages::MessageHandle},
    network::TcpMessageItem,
    Account,
};

use super::{EndpointDto, SocketHandle};

pub struct TcpMessageItemHandle(TcpMessageItem);

impl TcpMessageItemHandle {
    pub fn new(msg: TcpMessageItem) -> *mut Self {
        Box::into_raw(Box::new(TcpMessageItemHandle(msg)))
    }
}

impl Deref for TcpMessageItemHandle {
    type Target = TcpMessageItem;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub extern "C" fn rsn_tcp_message_item_empty() -> *mut TcpMessageItemHandle {
    TcpMessageItemHandle::new(TcpMessageItem::new())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_message_item_create(
    message: *const MessageHandle,
    endpoint: *const EndpointDto,
    node_id: *const u8,
    socket: *const SocketHandle,
) -> *mut TcpMessageItemHandle {
    let message = if message.is_null() {
        None
    } else {
        Some((*message).clone_box())
    };
    let endpoint = SocketAddr::from(&*endpoint);
    let node_id = Account::from_ptr(node_id);
    let socket = if socket.is_null() {
        None
    } else {
        Some((*socket).deref().clone())
    };
    TcpMessageItemHandle::new(TcpMessageItem {
        message,
        endpoint,
        node_id,
        socket,
    })
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_message_item_clone(
    handle: *mut TcpMessageItemHandle,
) -> *mut TcpMessageItemHandle {
    Box::into_raw(Box::new(TcpMessageItemHandle((*handle).0.clone())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_message_item_destroy(handle: *mut TcpMessageItemHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_message_item_endpoint(
    handle: *mut TcpMessageItemHandle,
    endpoint: *mut EndpointDto,
) {
    *endpoint = EndpointDto::from(&(*handle).0.endpoint);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_message_item_node_id(
    handle: *mut TcpMessageItemHandle,
    node_id: *mut u8,
) {
    copy_account_bytes((*handle).0.node_id, node_id);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_message_item_message(
    handle: *mut TcpMessageItemHandle,
) -> *mut MessageHandle {
    match &(*handle).0.message {
        Some(msg) => MessageHandle::new(msg.clone_box()),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_message_item_socket(
    handle: *mut TcpMessageItemHandle,
) -> *mut SocketHandle {
    match &(*handle).0.socket {
        Some(socket) => SocketHandle::new(socket.clone()),
        None => std::ptr::null_mut(),
    }
}
