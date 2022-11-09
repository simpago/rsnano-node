mod account_info;
pub use account_info::AccountInfoHandle;

mod blocks;
pub use blocks::*;

mod random_pool;

pub mod messages;

mod epoch;
pub use epoch::EpochsHandle;

mod unchecked_info;
pub(crate) use unchecked_info::*;

use rand::{thread_rng, Rng};
use std::{ffi::CStr, net::Ipv6Addr, os::raw::c_char, slice};

use crate::core::{
    deterministic_key, ip_address_hash_raw, sign_message, validate_message, validate_message_batch,
    Account, Difficulty, KeyPair, PublicKey, RawKey, Signature,
};

use super::copy_raw_key_bytes;

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
    priv_key: *const u8,
    pub_key: *const u8,
    message: *const u8,
    len: usize,
    signature: *mut u8,
) -> i32 {
    let private_key = RawKey::from_ptr(priv_key);
    let public_key = PublicKey::from_ptr(pub_key);
    let data = std::slice::from_raw_parts(message, len);
    match sign_message(&private_key, &public_key, data) {
        Ok(sig) => {
            let signature = slice::from_raw_parts_mut(signature, 64);
            signature.copy_from_slice(sig.as_bytes());
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

#[no_mangle]
pub unsafe extern "C" fn rsn_pub_key(raw_key: *const u8, pub_key: *mut u8) {
    let raw_key = RawKey::from_ptr(raw_key);
    let p = PublicKey::try_from(&raw_key).unwrap();
    let bytes = std::slice::from_raw_parts_mut(pub_key, 32);
    bytes.copy_from_slice(p.as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_keypair_create(prv_key: *mut u8, pub_key: *mut u8) {
    let pair = KeyPair::new();
    slice::from_raw_parts_mut(prv_key, 32).copy_from_slice(pair.private_key().as_bytes());
    slice::from_raw_parts_mut(pub_key, 32).copy_from_slice(pair.public_key().as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_keypair_create_from_prv_key(prv_key: *const u8, pub_key: *mut u8) {
    let pair = KeyPair::from_priv_key_bytes(slice::from_raw_parts(prv_key, 32)).unwrap();
    slice::from_raw_parts_mut(pub_key, 32).copy_from_slice(pair.public_key().as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_keypair_create_from_hex_str(
    prv_hex: *const c_char,
    prv_key: *mut u8,
    pub_key: *mut u8,
) {
    let pair = KeyPair::from_priv_key_hex(CStr::from_ptr(prv_hex).to_str().unwrap()).unwrap();
    slice::from_raw_parts_mut(prv_key, 32).copy_from_slice(pair.private_key().as_bytes());
    slice::from_raw_parts_mut(pub_key, 32).copy_from_slice(pair.public_key().as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_random_wallet_id(result: *mut u8) {
    let secret = thread_rng().gen::<[u8; 32]>();
    let keys = KeyPair::from_priv_key_bytes(&secret).unwrap();
    slice::from_raw_parts_mut(result, 32).copy_from_slice(keys.public_key().as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ip_address_hash_raw(address: *const u8, port: u16) -> u64 {
    let bytes: [u8; 16] = std::slice::from_raw_parts(address, 16).try_into().unwrap();
    let v6 = Ipv6Addr::from(bytes);
    ip_address_hash_raw(&v6, port)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_deterministic_key(seed: *const u8, index: u32, result: *mut u8) {
    let key = deterministic_key(&RawKey::from_ptr(seed), index);
    copy_raw_key_bytes(key, result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_raw_key_encrypt(
    value: *mut u8,
    cleartext: *const u8,
    key: *const u8,
    iv: *const u8,
) {
    let cleartext = RawKey::from_ptr(cleartext);
    let key = RawKey::from_ptr(key);
    let iv = slice::from_raw_parts(iv, 16).try_into().unwrap();
    let encrypted = cleartext.encrypt(&key, &iv);
    copy_raw_key_bytes(encrypted, value);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_raw_key_decrypt(
    value: *mut u8,
    ciphertext: *const u8,
    key: *const u8,
    iv: *const u8,
) {
    let ciphertext = RawKey::from_ptr(ciphertext);
    let key = RawKey::from_ptr(key);
    let iv = slice::from_raw_parts(iv, 16).try_into().unwrap();
    let decrypted = ciphertext.decrypt(&key, &iv);
    copy_raw_key_bytes(decrypted, value);
}
