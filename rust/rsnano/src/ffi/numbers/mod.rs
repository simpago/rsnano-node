mod account_info;

use std::{ffi::CStr, os::raw::c_char};

pub use account_info::AccountInfoHandle;

use crate::numbers::{
    sign_message, validate_message, validate_message_batch, Account, Difficulty, PublicKey, RawKey,
    Signature,
};

#[no_mangle]
pub extern "C" fn rsn_difficulty_to_multiplier(difficulty: u64, base_difficulty: u64) -> f64 {
    Difficulty::to_multiplier(difficulty, base_difficulty)
}

#[no_mangle]
pub extern "C" fn rsn_difficulty_from_multiplier(multiplier: f64, base_difficulty: u64) -> u64 {
    Difficulty::from_multiplier(multiplier, base_difficulty)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_account_encode(bytes: *const [u8; 32], result: *mut [u8; 65]) {
    let encoded = Account::from_bytes(*bytes).encode_account();
    (*result).copy_from_slice(encoded.as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_account_decode(input: *const c_char, result: *mut [u8; 32]) -> i32 {
    let input_string = match CStr::from_ptr(input).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let account = match Account::decode_account(input_string) {
        Ok(a) => a,
        Err(_) => return -1,
    };

    (*result).copy_from_slice(account.as_bytes());
    0
}

#[no_mangle]
pub unsafe extern "C" fn rsn_sign_message(
    priv_key: &[u8; 32],
    pub_key: &[u8; 32],
    message: *const u8,
    len: usize,
    signature: *mut [u8; 64],
) -> i32 {
    let private_key = RawKey::from_bytes(*priv_key);
    let public_key = PublicKey::from_bytes(*pub_key);
    let data = std::slice::from_raw_parts(message, len);
    match sign_message(&private_key, &public_key, data) {
        Ok(sig) => {
            *signature = sig.to_be_bytes();
            0
        }
        Err(_) => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_validate_message(
    pub_key: &[u8; 32],
    message: *const u8,
    len: usize,
    signature: &[u8; 64],
) -> bool {
    let public_key = PublicKey::from_bytes(*pub_key);
    let message = std::slice::from_raw_parts(message, len);
    let signature = Signature::from_bytes(*signature);
    validate_message(&public_key, message, &signature).is_err()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_validate_batch(
    messages: *const *const u8,
    message_lengths: *const usize,
    public_keys: *const *const u8,
    signatures: *const *const u8,
    num: usize,
    valid: *mut i32,
) -> bool {
    let message_lengths = std::slice::from_raw_parts(message_lengths, num);

    let messages = std::slice::from_raw_parts(messages, num)
        .iter()
        .enumerate()
        .map(|(i, &m)| {
            let msg = std::slice::from_raw_parts(m, message_lengths[i]);
            msg.to_owned()
        })
        .collect::<Vec<_>>();

    let mut key_buffer = [0_u8; 32];
    let public_keys = std::slice::from_raw_parts(public_keys, num)
        .iter()
        .map(|&bytes| {
            let bytes = std::slice::from_raw_parts(bytes, 32);
            key_buffer.copy_from_slice(bytes);
            PublicKey::from_bytes(key_buffer)
        })
        .collect::<Vec<_>>();

    let mut sig_buffer = [0_u8; 64];
    let signatures = std::slice::from_raw_parts(signatures, num)
        .iter()
        .map(|&bytes| {
            let bytes = std::slice::from_raw_parts(bytes, 64);
            sig_buffer.copy_from_slice(bytes);
            Signature::from_bytes(sig_buffer)
        })
        .collect::<Vec<_>>();

    let valid = std::slice::from_raw_parts_mut(valid, num);

    validate_message_batch(&messages, &public_keys, &signatures, valid);
    true
}
