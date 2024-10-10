mod account_info;
mod blocks;
mod epoch;
mod random_pool;
mod unchecked_info;

pub use account_info::AccountInfoHandle;
pub use blocks::*;
pub use epoch::EpochsHandle;
use rsnano_core::{
    deterministic_key, sign_message, validate_message, Account, BlockHash, DifficultyV1, KeyPair,
    PublicKey, RawKey, Signature, WalletId,
};
use rsnano_node::utils::ip_address_hash_raw;
use std::{ffi::CStr, net::Ipv6Addr, os::raw::c_char, slice};
pub use unchecked_info::*;

#[no_mangle]
pub extern "C" fn rsn_difficulty_to_multiplier(difficulty: u64, base_difficulty: u64) -> f64 {
    DifficultyV1::to_multiplier(difficulty, base_difficulty)
}

#[no_mangle]
pub extern "C" fn rsn_difficulty_from_multiplier(multiplier: f64, base_difficulty: u64) -> u64 {
    DifficultyV1::from_multiplier(multiplier, base_difficulty)
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
    _pub_key: *const u8,
    message: *const u8,
    len: usize,
    signature: *mut u8,
) -> i32 {
    let private_key = RawKey::from_ptr(priv_key);
    let data = if message.is_null() {
        &[]
    } else {
        std::slice::from_raw_parts(message, len)
    };
    let sig = sign_message(&private_key, data);
    let signature = slice::from_raw_parts_mut(signature, 64);
    signature.copy_from_slice(sig.as_bytes());
    0
}

#[no_mangle]
pub unsafe extern "C" fn rsn_validate_message(
    pub_key: &[u8; 32],
    message: *const u8,
    len: usize,
    signature: &[u8; 64],
) -> bool {
    let public_key = PublicKey::from_bytes(*pub_key);
    let message = if message.is_null() {
        &[]
    } else {
        std::slice::from_raw_parts(message, len)
    };
    let signature = Signature::from_bytes(*signature);
    validate_message(&public_key, message, &signature).is_err()
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
    let wallet_id = WalletId::random();
    slice::from_raw_parts_mut(result, 32).copy_from_slice(wallet_id.as_bytes());
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
    key.copy_bytes(result);
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
    encrypted.copy_bytes(value);
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
    decrypted.copy_bytes(value);
}

//
// BlockHashVec
//

pub struct BlockHashVecHandle(pub Vec<BlockHash>);

#[no_mangle]
pub extern "C" fn rsn_block_hash_vec_create() -> *mut BlockHashVecHandle {
    Box::into_raw(Box::new(BlockHashVecHandle(Vec::new())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_hash_vec_clone(
    handle: *const BlockHashVecHandle,
) -> *mut BlockHashVecHandle {
    Box::into_raw(Box::new(BlockHashVecHandle((*handle).0.clone())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_hash_vec_destroy(handle: *mut BlockHashVecHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_hash_vec_size(handle: *mut BlockHashVecHandle) -> usize {
    (*handle).0.len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_hash_vec_push(handle: *mut BlockHashVecHandle, hash: *const u8) {
    (*handle).0.push(BlockHash::from_ptr(hash));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_hash_vec_clear(handle: *mut BlockHashVecHandle) {
    (*handle).0.clear();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_hash_vec_assign_range(
    destination: *mut BlockHashVecHandle,
    source: *const BlockHashVecHandle,
    start: usize,
    end: usize,
) {
    (*destination).0.clear();
    (*destination).0.extend_from_slice(&(*source).0[start..end]);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_hash_vec_truncate(
    handle: *mut BlockHashVecHandle,
    new_size: usize,
) {
    (*handle).0.truncate(new_size);
}
