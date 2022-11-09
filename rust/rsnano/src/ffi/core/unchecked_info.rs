use std::{ffi::c_void, ops::Deref};

use num::FromPrimitive;

use crate::{
    core::{Account, UncheckedInfo},
    utils::{Deserialize, Serialize},
};

use crate::ffi::{core::BlockHandle, utils::FfiStream};

pub struct UncheckedInfoHandle(UncheckedInfo);

impl UncheckedInfoHandle {
    pub(crate) fn new(info: UncheckedInfo) -> Self {
        Self(info)
    }
}

impl Deref for UncheckedInfoHandle {
    type Target = UncheckedInfo;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub extern "C" fn rsn_unchecked_info_create() -> *mut UncheckedInfoHandle {
    let info = UncheckedInfo::null();
    Box::into_raw(Box::new(UncheckedInfoHandle(info)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_unchecked_info_create2(
    block: *const BlockHandle,
    account: *const u8,
    verified: u8,
) -> *mut UncheckedInfoHandle {
    let block = (*block).block.clone();
    let mut bytes = [0; 32];
    bytes.copy_from_slice(std::slice::from_raw_parts(account, 32));
    let account = Account::from_bytes(bytes);
    let info = UncheckedInfo::new(block, &account, FromPrimitive::from_u8(verified).unwrap());
    Box::into_raw(Box::new(UncheckedInfoHandle(info)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_unchecked_info_clone(
    handle: *const UncheckedInfoHandle,
) -> *mut UncheckedInfoHandle {
    Box::into_raw(Box::new(UncheckedInfoHandle((*handle).0.clone())))
}

#[no_mangle]
pub extern "C" fn rsn_unchecked_info_destroy(handle: *mut UncheckedInfoHandle) {
    drop(unsafe { Box::from_raw(handle) });
}

#[no_mangle]
pub unsafe extern "C" fn rsn_unchecked_info_block(
    handle: *const UncheckedInfoHandle,
) -> *mut BlockHandle {
    Box::into_raw(Box::new(BlockHandle {
        block: (*handle).0.block.as_ref().unwrap().clone(),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_unchecked_info_modified(handle: *const UncheckedInfoHandle) -> u64 {
    (*handle).0.modified
}

#[no_mangle]
pub unsafe extern "C" fn rsn_unchecked_info_account(
    handle: *const UncheckedInfoHandle,
    result: *mut u8,
) {
    std::slice::from_raw_parts_mut(result, 32).copy_from_slice((*handle).0.account.as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_unchecked_info_verified(handle: *const UncheckedInfoHandle) -> u8 {
    (*handle).0.verified as u8
}

#[no_mangle]
pub unsafe extern "C" fn rsn_unchecked_info_verified_set(
    handle: *mut UncheckedInfoHandle,
    verified: u8,
) {
    (*handle).0.verified = FromPrimitive::from_u8(verified).unwrap();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_unchecked_info_serialize(
    handle: *mut UncheckedInfoHandle,
    stream: *mut c_void,
) -> bool {
    let mut stream = FfiStream::new(stream);
    (*handle).0.serialize(&mut stream).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_unchecked_info_deserialize(
    handle: *mut UncheckedInfoHandle,
    stream: *mut c_void,
) -> bool {
    let mut stream = FfiStream::new(stream);
    match UncheckedInfo::deserialize(&mut stream) {
        Ok(info) => {
            (*handle).0 = info;
            true
        }
        Err(_) => false,
    }
}
