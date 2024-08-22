mod async_runtime;
mod atomics;
mod container_info;
mod latch;
mod logging;
mod stream;
mod thread_pool;
mod timer;

use crate::{transport::EndpointDto, VoidPointerCallback};
pub use async_runtime::AsyncRuntimeHandle;
pub use container_info::*;
use rsnano_network::utils::{
    ipv4_address_or_ipv6_subnet, map_address_to_subnetwork, reserved_address,
};
use std::{
    ffi::c_void,
    net::{Ipv6Addr, SocketAddrV6},
    time::Instant,
};
pub use stream::FfiStream;
pub use thread_pool::ThreadPoolHandle;

pub struct ContextWrapper {
    context: *mut c_void,
    drop_context: VoidPointerCallback,
}

impl ContextWrapper {
    pub fn new(context: *mut c_void, drop_context: VoidPointerCallback) -> Self {
        Self {
            context,
            drop_context,
        }
    }

    pub fn get_context(&self) -> *mut c_void {
        self.context
    }
}

unsafe impl Send for ContextWrapper {}
unsafe impl Sync for ContextWrapper {}

impl Drop for ContextWrapper {
    fn drop(&mut self) {
        unsafe {
            (self.drop_context)(self.context);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_map_address_to_subnetwork(ipv6_bytes: *const u8, result: *mut u8) {
    let input = ptr_into_ipv6addr(ipv6_bytes);
    let output = map_address_to_subnetwork(&input);
    copy_ipv6addr_bytes(output, result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ipv4_address_or_ipv6_subnet(ipv6_bytes: *const u8, result: *mut u8) {
    let input = ptr_into_ipv6addr(ipv6_bytes);
    let output = ipv4_address_or_ipv6_subnet(&input);
    copy_ipv6addr_bytes(output, result);
}

pub(crate) unsafe fn copy_ipv6addr_bytes(res: Ipv6Addr, target: *mut u8) {
    let result_slice = std::slice::from_raw_parts_mut(target, 16);
    result_slice.copy_from_slice(&res.octets());
}

pub(crate) unsafe fn ptr_into_ipv6addr(ipv6_bytes: *const u8) -> Ipv6Addr {
    let octets: [u8; 16] = std::slice::from_raw_parts(ipv6_bytes, 16)
        .try_into()
        .unwrap();
    Ipv6Addr::from(octets)
}

#[no_mangle]
pub extern "C" fn rsn_reserved_address(endpoint: &EndpointDto, allow_local_peers: bool) -> bool {
    let endpoint = SocketAddrV6::from(endpoint);
    reserved_address(&endpoint, allow_local_peers)
}

pub struct InstantHandle(pub Instant);

#[no_mangle]
pub extern "C" fn rsn_instant_now() -> *mut InstantHandle {
    Box::into_raw(Box::new(InstantHandle(Instant::now())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_instant_destroy(handle: *mut InstantHandle) {
    drop(Box::from_raw(handle));
}

#[no_mangle]
pub extern "C" fn rsn_instant_elapsed_ms(handle: &InstantHandle) -> u64 {
    handle.0.elapsed().as_millis() as u64
}
