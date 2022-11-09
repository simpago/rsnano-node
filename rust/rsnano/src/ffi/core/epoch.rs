use crate::core::{Epochs, Link, PublicKey};
use num::FromPrimitive;
use std::convert::TryInto;

pub struct EpochsHandle {
    pub epochs: Epochs,
}

#[no_mangle]
pub extern "C" fn rsn_epochs_create() -> *mut EpochsHandle {
    Box::into_raw(Box::new(EpochsHandle {
        epochs: Epochs::new(),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_epochs_destroy(handle: *mut EpochsHandle) {
    drop(Box::from_raw(handle));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_epochs_add(
    handle: *mut EpochsHandle,
    epoch: u8,
    signer: *const u8,
    link: *const u8,
) {
    let epoch = FromPrimitive::from_u8(epoch).unwrap();
    let signer = PublicKey::from_bytes(std::slice::from_raw_parts(signer, 32).try_into().unwrap());
    let link = Link::from_bytes(std::slice::from_raw_parts(link, 32).try_into().unwrap());
    (*handle).epochs.add(epoch, signer, link);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_epochs_is_epoch_link(
    handle: *const EpochsHandle,
    link: *const u8,
) -> bool {
    let link = Link::from_bytes(std::slice::from_raw_parts(link, 32).try_into().unwrap());
    (*handle).epochs.is_epoch_link(&link)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_epochs_link(handle: *const EpochsHandle, epoch: u8, link: *mut u8) {
    let epoch = FromPrimitive::from_u8(epoch).unwrap();
    let l = (*handle).epochs.link(epoch).unwrap();
    let link = std::slice::from_raw_parts_mut(link, 32);
    link.copy_from_slice(l.as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_epochs_signer(
    handle: *const EpochsHandle,
    epoch: u8,
    signer: *mut u8,
) {
    let epoch = FromPrimitive::from_u8(epoch).unwrap();
    let key = (*handle).epochs.signer(epoch).unwrap();
    let signer = std::slice::from_raw_parts_mut(signer, 32);
    signer.copy_from_slice(key.as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_epochs_epoch(handle: *const EpochsHandle, link: *const u8) -> u8 {
    let link = Link::from_bytes(std::slice::from_raw_parts(link, 32).try_into().unwrap());
    let epoch = (*handle).epochs.epoch(&link).unwrap();
    epoch as u8
}
