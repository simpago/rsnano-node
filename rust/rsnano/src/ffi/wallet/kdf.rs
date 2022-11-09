use std::{
    ffi::{c_char, CStr},
    ops::Deref,
    slice,
};

use crate::{ffi::copy_raw_key_bytes, wallet::KeyDerivationFunction};
pub struct KdfHandle(KeyDerivationFunction);

impl Deref for KdfHandle {
    type Target = KeyDerivationFunction;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub extern "C" fn rsn_kdf_create(kdf_work: u32) -> *mut KdfHandle {
    Box::into_raw(Box::new(KdfHandle(KeyDerivationFunction::new(kdf_work))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_kdf_destroy(handle: *mut KdfHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_kdf_phs(
    handle: *mut KdfHandle,
    result: *mut u8,
    password: *const c_char,
    salt: *const u8,
) {
    let password = CStr::from_ptr(password).to_str().unwrap();
    let salt = slice::from_raw_parts(salt, 32).try_into().unwrap();
    let key = (*handle).0.hash_password(password, salt);
    copy_raw_key_bytes(key, result);
}
