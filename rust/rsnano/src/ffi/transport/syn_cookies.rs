use crate::{
    core::{Account, Signature},
    transport::SynCookies,
};
use std::{net::SocketAddr, ops::Deref, sync::Arc, time::Duration};

use super::EndpointDto;

pub struct SynCookiesHandle(Arc<SynCookies>);

impl Deref for SynCookiesHandle {
    type Target = Arc<SynCookies>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub extern "C" fn rsn_syn_cookies_create(max_cookies_per_ip: usize) -> *mut SynCookiesHandle {
    Box::into_raw(Box::new(SynCookiesHandle(Arc::new(SynCookies::new(
        max_cookies_per_ip,
    )))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_syn_cookies_destroy(handle: *mut SynCookiesHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_syn_cookies_assign(
    handle: *mut SynCookiesHandle,
    endpoint: *const EndpointDto,
    result: *mut u8,
) -> bool {
    match (*handle).0.assign(&SocketAddr::from(&*endpoint)) {
        Some(cookie) => {
            let result = std::slice::from_raw_parts_mut(result, 32);
            result.copy_from_slice(&cookie);
            true
        }
        None => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_syn_cookies_validate(
    handle: *mut SynCookiesHandle,
    endpoint: *const EndpointDto,
    node_id: *const u8,
    signature: *const u8,
) -> bool {
    let endpoint = SocketAddr::from(&*endpoint);
    let node_id = Account::from_ptr(node_id);
    let signature = Signature::from_ptr(signature);
    (*handle)
        .0
        .validate(&endpoint, &node_id, &signature)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_syn_cookies_purge(handle: *mut SynCookiesHandle, cutoff_s: u32) {
    (*handle).0.purge(Duration::from_secs(cutoff_s as u64));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_syn_cookies_cookies_count(handle: *mut SynCookiesHandle) -> usize {
    (*handle).0.cookies_count()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_syn_cookies_cookies_per_ip_count(
    handle: *mut SynCookiesHandle,
) -> usize {
    (*handle).0.cookies_per_ip_count()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_syn_cookies_cookie_info_size() -> usize {
    SynCookies::cookie_info_size()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_syn_cookies_cookies_per_ip_size() -> usize {
    SynCookies::cookies_per_ip_size()
}
