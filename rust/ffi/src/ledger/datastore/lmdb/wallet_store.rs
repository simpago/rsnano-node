use super::{iterator::LmdbIteratorHandle, TransactionHandle};
use crate::{wallets::kdf::KdfHandle, StringDto};
use rsnano_core::{PublicKey, RawKey};
use rsnano_store_lmdb::{LmdbWalletStore, WalletValue};
use std::{
    ffi::{c_char, CStr},
    ops::Deref,
    path::PathBuf,
    ptr,
    sync::Arc,
};

pub struct LmdbWalletStoreHandle(pub Arc<LmdbWalletStore>);

impl Deref for LmdbWalletStoreHandle {
    type Target = Arc<LmdbWalletStore>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

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
    let representative = PublicKey::from_ptr(representative);
    if let Ok(store) = LmdbWalletStore::new(
        fanout,
        (*kdf).deref().clone(),
        (*txn).as_write_txn(),
        &representative,
        &wallet,
    ) {
        Box::into_raw(Box::new(LmdbWalletStoreHandle(Arc::new(store))))
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
        Box::into_raw(Box::new(LmdbWalletStoreHandle(Arc::new(store))))
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
    value.copy_bytes(result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_salt(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    result: *mut u8,
) {
    let value = (*handle).0.salt((*txn).as_txn());
    value.copy_bytes(result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_wallet_key(
    handle: *mut LmdbWalletStoreHandle,
    prv_key: *mut u8,
    txn: *mut TransactionHandle,
) {
    let key = (*handle).0.wallet_key((*txn).as_txn());
    key.copy_bytes(prv_key);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_seed(
    handle: *mut LmdbWalletStoreHandle,
    prv_key: *mut u8,
    txn: *mut TransactionHandle,
) {
    let key = (*handle).0.seed((*txn).as_txn());
    key.copy_bytes(prv_key);
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
    key.copy_bytes(prv);
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
pub unsafe extern "C" fn rsn_lmdb_wallet_store_erase(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    account: *const u8,
) {
    (*handle)
        .0
        .erase((*txn).as_write_txn(), &PublicKey::from_ptr(account));
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
    result.copy_bytes(key);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_find(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    account: *const u8,
) -> *mut LmdbIteratorHandle {
    let iterator = (*handle)
        .0
        .find((*txn).as_txn(), &PublicKey::from_ptr(account));
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
    result.copy_bytes(key);
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
pub unsafe extern "C" fn rsn_lmdb_wallet_store_representative(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    account: *mut u8,
) {
    let rep = (*handle).0.representative((*txn).as_txn());
    rep.copy_bytes(account);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_representative_set(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    representative: *const u8,
) {
    (*handle)
        .0
        .representative_set((*txn).as_write_txn(), &PublicKey::from_ptr(representative));
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
    public_key.copy_bytes(pub_key);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallet_store_insert_watch(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    pub_key: *const u8,
) -> bool {
    (*handle)
        .0
        .insert_watch((*txn).as_write_txn(), &PublicKey::from_ptr(pub_key))
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
        .fetch((*txn).as_txn(), &PublicKey::from_ptr(pub_key))
    {
        Ok(prv) => {
            prv.copy_bytes(prv_key);
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
pub unsafe extern "C" fn rsn_lmdb_wallet_store_move(
    handle: *mut LmdbWalletStoreHandle,
    txn: *mut TransactionHandle,
    other: *mut LmdbWalletStoreHandle,
    keys: *const u8,
    count: usize,
) -> bool {
    let keys: *const [u8; 32] = std::mem::transmute(keys);
    let keys = if keys.is_null() {
        &[]
    } else {
        std::slice::from_raw_parts(keys, count)
    };
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
    k.copy_bytes(password);
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
