use crate::transport::EndpointDto;
use rsnano_network::TcpListener;
use std::{ops::Deref, sync::Arc};
use tracing::debug;

pub struct TcpListenerHandle(pub Arc<TcpListener>);

impl Deref for TcpListenerHandle {
    type Target = Arc<TcpListener>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_listener_destroy(handle: *mut TcpListenerHandle) {
    debug!("calling TCP listener destroy");
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub extern "C" fn rsn_tcp_listener_endpoint(handle: &TcpListenerHandle, result: &mut EndpointDto) {
    *result = handle.0.local_address().into()
}
