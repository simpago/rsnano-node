use crate::{messages::TelemetryDataHandle, transport::EndpointDto};
use rsnano_messages::TelemetryData;
use rsnano_node::Telemetry;
use std::{net::SocketAddrV6, ops::Deref, sync::Arc};

pub struct TelemetryHandle(pub Arc<Telemetry>);

impl Deref for TelemetryHandle {
    type Target = Arc<Telemetry>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_telemetry_destroy(handle: *mut TelemetryHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_telemetry_local_telemetry(
    handle: &TelemetryHandle,
) -> *mut TelemetryDataHandle {
    TelemetryDataHandle::new(handle.0.local_telemetry())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_telemetry_trigger(handle: &TelemetryHandle) {
    handle.0.trigger();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_telemetry_len(handle: &TelemetryHandle) -> usize {
    handle.0.len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_telemetry_get_telemetry(
    handle: &TelemetryHandle,
    endpoint: &EndpointDto,
) -> *mut TelemetryDataHandle {
    match handle.0.get_telemetry(&endpoint.into()) {
        Some(data) => TelemetryDataHandle::new(data),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_telemetry_get_all(
    handle: &TelemetryHandle,
) -> *mut TelemetryDataMapHandle {
    Box::into_raw(Box::new(TelemetryDataMapHandle(
        handle.0.get_all_telemetries().drain().collect(),
    )))
}

pub struct TelemetryDataMapHandle(Vec<(SocketAddrV6, TelemetryData)>);

#[no_mangle]
pub unsafe extern "C" fn rsn_telemetry_data_map_destroy(handle: *mut TelemetryDataMapHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub extern "C" fn rsn_telemetry_data_map_len(handle: &TelemetryDataMapHandle) -> usize {
    handle.0.len()
}

#[no_mangle]
pub extern "C" fn rsn_telemetry_data_map_get(
    handle: &TelemetryDataMapHandle,
    index: usize,
    endpoint: &mut EndpointDto,
) -> *mut TelemetryDataHandle {
    let (ep, data) = &handle.0[index];
    *endpoint = ep.into();
    TelemetryDataHandle::new(data.clone())
}
