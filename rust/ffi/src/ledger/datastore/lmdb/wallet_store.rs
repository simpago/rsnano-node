use std::{
    ffi::{c_char, CStr},
    ops::Deref,
    path::PathBuf,
    ptr,
};

use crate::{
    copy_account_bytes, copy_public_key_bytes, copy_raw_key_bytes, wallet::kdf::KdfHandle,
    StringDto, U256ArrayDto,
};
use rsnano_core::{Account, PublicKey, RawKey};
use rsnano_store_lmdb::{EnvironmentWrapper, LmdbWalletStore, WalletValue};

use super::{iterator::LmdbIteratorHandle, TransactionHandle};

pub struct LmdbWalletStoreHandle(LmdbWalletStore);

#[repr(C)]
pub struct WalletValueDto {
    pub key: [u8; 32],
    pub work: u64,
}

impl From<WalletValue> for WalletValueDto {
    fn from(value: WalletValue) -> Self {
        WalletValueDto {
            key: *value.key.as_bytes(),
            work: value.work,
        }
    }
}

impl From<&WalletValueDto> for WalletValue {
    fn from(dto: &WalletValueDto) -> Self {
        WalletValue::new(RawKey::from_bytes(dto.key), dto.work)
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_create(
    fanout: usize,
    kdf: *const KdfHandle,
    txn: *mut TransactionHandle,
    representative: *const u8,
    wallet: *const c_char,
) -> *mut LmdbWalletStoreHandle {
    let wallet = CStr::from_ptr(wallet).to_str().unwrap();
    let wallet = PathBuf::from(wallet);
    let representative = Account::from_ptr(representative);
    if let Ok(store) = LmdbWalletStore::new(
        fanout,
        (*kdf).deref().clone(),
        (*txn).as_write_txn(),
        &representative,
        &wallet,
    ) {
        Box::into_raw(Box::new(LmdbWalletStoreHandle(store)))
    } else {
        ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_create2(
    fanout: usize,
    kdf: *const KdfHandle,
    txn: *mut TransactionHandle,
    wallet: *const c_char,
    json: *const c_char,
) -> *mut LmdbWalletStoreHandle {
    let wallet = CStr::from_ptr(wallet).to_str().unwrap();
    let json = CStr::from_ptr(json).to_str().unwrap();
    let wallet = PathBuf::from(wallet);
    if let Ok(store) = LmdbWalletStore::new_from_json(
        fanout,
        (*kdf).deref().clone(),
        (*txn).as_write_txn(),
        &wallet,
        json,
    ) {
        Box::into_raw(Box::new(LmdbWalletStoreHandle(store)))
    } else {
        ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_destroy(handle: *mut LmdbWalletStoreHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_check(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    result: *mut u8,
) {
    let value = (*handle).0.check((*txn).as_txn());
    copy_raw_key_bytes(value, result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_salt(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    result: *mut u8,
) {
    let value = (*handle).0.salt((*txn).as_txn());
    copy_raw_key_bytes(value, result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_wallet_key(
    handle: *mut LmdbWalletStoreHandle,
    prv_key: *mut u8,
    txn: *mut TransactionHandle,
) {
    let key = (*handle).0.wallet_key((*txn).as_txn());
    copy_raw_key_bytes(key, prv_key);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_seed(
    handle: *mut LmdbWalletStoreHandle,
    prv_key: *mut u8,
    txn: *mut TransactionHandle,
) {
    let key = (*handle).0.seed((*txn).as_txn());
    copy_raw_key_bytes(key, prv_key);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_seed_set(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    prv_key: *const u8,
) {
    (*handle)
        .0
        .set_seed((*txn).as_write_txn(), &RawKey::from_ptr(prv_key));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_deterministic_index_get(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
) -> u32 {
    (*handle).0.deterministic_index_get((*txn).as_txn())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_deterministic_index_set(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    index: u32,
) {
    (*handle)
        .0
        .deterministic_index_set((*txn).as_write_txn(), index);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_valid_password(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
) -> bool {
    (*handle).0.valid_password((*txn).as_txn())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_derive_key(
    handle: *mut LmdbWalletStoreHandle,
    prv: *mut u8,
    txn: *mut TransactionHandle,
    password: *const c_char,
) {
    let password = CStr::from_ptr(password).to_str().unwrap();
    let key = (*handle).0.derive_key((*txn).as_txn(), password);
    copy_raw_key_bytes(key, prv);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_rekey(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    password: *const c_char,
) -> bool {
    let password = CStr::from_ptr(password).to_str().unwrap();
    (*handle).0.rekey((*txn).as_write_txn(), password).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_begin(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
) -> *mut LmdbIteratorHandle {
    let iterator = (*handle).0.begin((*txn).as_txn());
    LmdbIteratorHandle::new2(iterator)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_begin_at_account(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    account: *const u8,
) -> *mut LmdbIteratorHandle {
    let iterator = (*handle)
        .0
        .begin_at_account((*txn).as_txn(), &Account::from_ptr(account));
    LmdbIteratorHandle::new2(iterator)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_erase(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    account: *const u8,
) {
    (*handle)
        .0
        .erase((*txn).as_write_txn(), &Account::from_ptr(account));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_key_type(value: *const WalletValueDto) -> u8 {
    LmdbWalletStore::<EnvironmentWrapper>::key_type(&WalletValue::from(&*value)) as u8
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_deterministic_clear(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
) {
    (*handle).0.deterministic_clear((*txn).as_write_txn());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_deterministic_key(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    index: u32,
    key: *mut u8,
) {
    let result = (*handle).0.deterministic_key((*txn).as_txn(), index);
    copy_raw_key_bytes(result, key);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_find(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    account: *const u8,
) -> *mut LmdbIteratorHandle {
    let iterator = (*handle)
        .0
        .find((*txn).as_txn(), &Account::from_ptr(account));
    LmdbIteratorHandle::new2(iterator)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_exists(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    key: *const u8,
) -> bool {
    (*handle)
        .0
        .exists((*txn).as_txn(), &PublicKey::from_ptr(key))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_deterministic_insert(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    key: *mut u8,
) {
    let result = (*handle).0.deterministic_insert((*txn).as_write_txn());
    copy_public_key_bytes(&result, key);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_deterministic_insert_at(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    index: u32,
    key: *mut u8,
) {
    let result = (*handle)
        .0
        .deterministic_insert_at((*txn).as_write_txn(), index);
    copy_public_key_bytes(&result, key);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_version(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
) -> u32 {
    (*handle).0.version((*txn).as_txn())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_attempt_password(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    password: *const c_char,
) -> bool {
    let password = CStr::from_ptr(password).to_str().unwrap();
    (*handle).0.attempt_password((*txn).as_txn(), password)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_lock(handle: *mut LmdbWalletStoreHandle) {
    (*handle).0.lock();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_accounts(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    result: *mut U256ArrayDto,
) {
    let accounts = (*handle).0.accounts((*txn).as_txn());
    let data = accounts.iter().map(|a| *a.as_bytes()).collect();
    (*result).initialize(data);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_representative(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    account: *mut u8,
) {
    let rep = (*handle).0.representative((*txn).as_txn());
    copy_account_bytes(rep, account);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_representative_set(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    representative: *const u8,
) {
    (*handle)
        .0
        .representative_set((*txn).as_write_txn(), &Account::from_ptr(representative));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_insert_adhoc(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    prv: *const u8,
    pub_key: *mut u8,
) {
    let public_key = (*handle)
        .0
        .insert_adhoc((*txn).as_write_txn(), &RawKey::from_ptr(prv));
    copy_public_key_bytes(&public_key, pub_key);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_insert_watch(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    pub_key: *const u8,
) -> bool {
    (*handle)
        .0
        .insert_watch((*txn).as_write_txn(), &Account::from_ptr(pub_key))
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_fetch(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    pub_key: *const u8,
    prv_key: *mut u8,
) -> bool {
    match (*handle)
        .0
        .fetch((*txn).as_txn(), &Account::from_ptr(pub_key))
    {
        Ok(prv) => {
            copy_raw_key_bytes(prv, prv_key);
            true
        }
        Err(_) => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_serialize_json(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    result: *mut StringDto,
) {
    let json = (*handle).0.serialize_json((*txn).as_txn());
    (*result) = json.into();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_write_backup(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    path: *const c_char,
) {
    let path = CStr::from_ptr(path).to_str().unwrap();
    let path = PathBuf::from(path);
    let _ = (*handle).0.write_backup((*txn).as_txn(), &path);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_move(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    other: *mut LmdbWalletStoreHandle,
    keys: *const u8,
    count: usize,
) -> bool {
    let keys: *const [u8; 32] = std::mem::transmute(keys);
    let keys = std::slice::from_raw_parts(keys, count);
    let keys: Vec<_> = keys
        .iter()
        .map(|bytes| PublicKey::from_bytes(*bytes))
        .collect();
    (*handle)
        .0
        .move_keys((*txn).as_write_txn(), &(*other).0, &keys)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_import(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    other: *mut LmdbWalletStoreHandle,
) -> bool {
    (*handle)
        .0
        .import((*txn).as_write_txn(), &(*other).0)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_work_get(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    pub_key: *const u8,
    work: *mut u64,
) -> bool {
    let pub_key = PublicKey::from_ptr(pub_key);
    match (*handle).0.work_get((*txn).as_txn(), &pub_key) {
        Ok(w) => {
            *work = w;
            true
        }
        Err(_) => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_work_put(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    pub_key: *const u8,
    work: u64,
) {
    let pub_key = PublicKey::from_ptr(pub_key);
    (*handle).0.work_put((*txn).as_write_txn(), &pub_key, work);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_destroy2(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
) {
    (*handle).0.destroy((*txn).as_write_txn());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_password(
    handle: *mut LmdbWalletStoreHandle,
    password: *mut u8,
) {
    let k = (*handle).0.fans.lock().unwrap().password.value();
    copy_raw_key_bytes(k, password);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_set_password(
    handle: *mut LmdbWalletStoreHandle,
    password: *const u8,
) {
    (*handle)
        .0
        .fans
        .lock()
        .unwrap()
        .password
        .value_set(RawKey::from_ptr(password));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_is_open(handle: *mut LmdbWalletStoreHandle) -> bool {
    (*handle).0.is_open()
}
