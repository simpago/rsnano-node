use std::{ops::Deref, sync::Arc};

use crate::{PublicKey, Signature, SignatureCheckSet, SignatureChecker};

#[repr(C)]
pub struct SignatureCheckSetDto {
    pub size: usize,
    pub messages: *const *const u8,
    pub message_lengths: *const usize,
    pub pub_keys: *const *const u8,
    pub signatures: *const *const u8,
    pub verifications: *mut i32,
}

pub struct SignatureCheckerHandle(Arc<SignatureChecker>);

impl Deref for SignatureCheckerHandle {
    type Target = Arc<SignatureChecker>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub extern "C" fn rsn_signature_checker_create(num_threads: usize) -> *mut SignatureCheckerHandle {
    Box::into_raw(Box::new(SignatureCheckerHandle(Arc::new(
        SignatureChecker::new(num_threads),
    ))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_signature_checker_destroy(handle: *mut SignatureCheckerHandle) {
    let bx = Box::from_raw(handle);
    bx.0.stop();
    drop(bx);
}

#[no_mangle]
pub extern "C" fn rsn_signature_checker_batch_size() -> usize {
    SignatureChecker::BATCH_SIZE
}

#[no_mangle]
pub unsafe extern "C" fn rsn_signature_checker_verify(
    handle: &SignatureCheckerHandle,
    check_set: *mut SignatureCheckSetDto,
) {
    let ffi_check_set = &mut *check_set;
    let mut check_set = into_check_set(ffi_check_set);
    handle.0.verify(&mut check_set);
    let valid = std::slice::from_raw_parts_mut(ffi_check_set.verifications, ffi_check_set.size);
    valid.copy_from_slice(&check_set.verifications);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_signature_checker_stop(handle: *mut SignatureCheckerHandle) {
    (*handle).0.stop();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_signature_checker_flush(handle: &SignatureCheckerHandle) -> bool {
    handle.0.flush().is_ok()
}

unsafe fn into_check_set(ffi_check_set: &SignatureCheckSetDto) -> SignatureCheckSet {
    let message_lengths =
        std::slice::from_raw_parts(ffi_check_set.message_lengths, ffi_check_set.size);

    let messages: Vec<Vec<u8>> =
        std::slice::from_raw_parts(ffi_check_set.messages, ffi_check_set.size)
            .iter()
            .enumerate()
            .map(|(i, &bytes)| {
                let msg = std::slice::from_raw_parts(bytes, message_lengths[i]);
                msg.to_vec()
            })
            .collect();

    let pub_keys = std::slice::from_raw_parts(ffi_check_set.pub_keys, ffi_check_set.size)
        .iter()
        .map(|&bytes| {
            let bytes = std::slice::from_raw_parts(bytes, 32);
            PublicKey::from_slice(bytes).unwrap()
        })
        .collect();

    let signatures = std::slice::from_raw_parts(ffi_check_set.signatures, ffi_check_set.size)
        .iter()
        .map(|&bytes| {
            let bytes = std::slice::from_raw_parts(bytes, 64);
            Signature::try_from_bytes(bytes).unwrap()
        })
        .collect();

    SignatureCheckSet::new(messages, pub_keys, signatures)
}
