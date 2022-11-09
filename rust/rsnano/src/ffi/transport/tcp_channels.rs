use std::sync::Arc;

use crate::transport::TcpChannels;

pub struct TcpChannelsHandle(Arc<TcpChannels>);

#[no_mangle]
pub extern "C" fn rsn_tcp_channels_create() -> *mut TcpChannelsHandle {
    Box::into_raw(Box::new(TcpChannelsHandle(Arc::new(TcpChannels::new()))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_destroy(handle: *mut TcpChannelsHandle) {
    drop(Box::from_raw(handle))
}
