use crate::{
    representatives::RepresentativeHandle, transport::TcpChannelsHandle, NetworkParamsDto,
};
use rsnano_node::{consensus::ConfirmationSolicitor, representatives::PeeredRep};
use std::ops::Deref;

use super::election::{ElectionHandle, ElectionLockHandle};

pub struct ConfirmationSolicitorHandle(ConfirmationSolicitor<'static>);

#[no_mangle]
pub extern "C" fn rsn_confirmation_solicitor_create(
    network_params: &NetworkParamsDto,
    tcp_channels: &'static TcpChannelsHandle,
) -> *mut ConfirmationSolicitorHandle {
    let solicitor =
        ConfirmationSolicitor::new(&network_params.try_into().unwrap(), tcp_channels.deref());
    Box::into_raw(Box::new(ConfirmationSolicitorHandle(solicitor)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_confirmation_solicitor_destroy(
    handle: *mut ConfirmationSolicitorHandle,
) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub extern "C" fn rsn_confirmation_solicitor_prepare(
    handle: &mut ConfirmationSolicitorHandle,
    reps: &RepresentativeVecHandle,
) {
    handle.0.prepare(&reps.0);
}

#[no_mangle]
pub extern "C" fn rsn_confirmation_solicitor_broadcast(
    handle: &mut ConfirmationSolicitorHandle,
    election_guard: &ElectionLockHandle,
) -> bool {
    handle
        .0
        .broadcast(&election_guard.0.as_ref().unwrap())
        .is_err()
}

#[no_mangle]
pub extern "C" fn rsn_confirmation_solicitor_add(
    handle: &mut ConfirmationSolicitorHandle,
    election: &ElectionHandle,
    election_guard: &ElectionLockHandle,
) -> bool {
    handle
        .0
        .add(&election.0, &election_guard.0.as_ref().unwrap())
}

#[no_mangle]
pub extern "C" fn rsn_confirmation_solicitor_flush(handle: &mut ConfirmationSolicitorHandle) {
    handle.0.flush()
}

pub struct RepresentativeVecHandle(Vec<PeeredRep>);

#[no_mangle]
pub extern "C" fn rsn_representative_vec_create() -> *mut RepresentativeVecHandle {
    Box::into_raw(Box::new(RepresentativeVecHandle(Vec::new())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_representative_vec_destroy(handle: *mut RepresentativeVecHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub extern "C" fn rsn_representative_vec_push(
    handle: &mut RepresentativeVecHandle,
    rep: &RepresentativeHandle,
) {
    handle.0.push(rep.0.clone())
}
