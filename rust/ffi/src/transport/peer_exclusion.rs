use rsnano_node::transport::PeerExclusion;
use std::{
    net::SocketAddrV6,
    sync::{Arc, Mutex},
};

use super::EndpointDto;

pub struct PeerExclusionHandle(pub Arc<Mutex<PeerExclusion>>);

#[no_mangle]
pub extern "C" fn rsn_peer_exclusion_create(max_size: usize) -> *mut PeerExclusionHandle {
    Box::into_raw(Box::new(PeerExclusionHandle(Arc::new(Mutex::new(
        PeerExclusion::with_max_size(max_size),
    )))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_peer_exclusion_destroy(handle: *mut PeerExclusionHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_peer_exclusion_add(
    handle: *mut PeerExclusionHandle,
    endpoint: *const EndpointDto,
) -> u64 {
    (*handle)
        .0
        .lock()
        .unwrap()
        .peer_misbehaved(&SocketAddrV6::from(&*endpoint))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_peer_exclusion_check(
    handle: *mut PeerExclusionHandle,
    endpoint: *const EndpointDto,
) -> bool {
    (*handle)
        .0
        .lock()
        .unwrap()
        .is_excluded(&SocketAddrV6::from(&*endpoint))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_peer_exclusion_score(
    handle: *mut PeerExclusionHandle,
    endpoint: *const EndpointDto,
) -> u64 {
    (*handle)
        .0
        .lock()
        .unwrap()
        .score(&SocketAddrV6::from(&*endpoint))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_peer_exclusion_contains(
    handle: *mut PeerExclusionHandle,
    endpoint: *const EndpointDto,
) -> bool {
    (*handle)
        .0
        .lock()
        .unwrap()
        .contains(&SocketAddrV6::from(&*endpoint))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_peer_exclusion_remove(
    handle: *mut PeerExclusionHandle,
    endpoint: *const EndpointDto,
) {
    (*handle)
        .0
        .lock()
        .unwrap()
        .remove(&SocketAddrV6::from(&*endpoint))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_peer_exclusion_size(handle: *mut PeerExclusionHandle) -> usize {
    (*handle).0.lock().unwrap().size()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_peer_exclusion_element_size() -> usize {
    PeerExclusion::element_size()
}
