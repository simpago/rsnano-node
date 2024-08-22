use crate::ledger::datastore::lmdb::LmdbWalletStoreHandle;
use rsnano_core::{Account, PublicKey};
use rsnano_node::wallets::Wallet;
use std::{
    collections::HashSet,
    ops::{Deref, DerefMut},
    sync::{Arc, MutexGuard},
};

pub struct WalletHandle(pub Arc<Wallet>);

impl Deref for WalletHandle {
    type Target = Arc<Wallet>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallet_store(
    handle: *const WalletHandle,
) -> *mut LmdbWalletStoreHandle {
    if handle.is_null() {
        std::ptr::null_mut()
    } else {
        Box::into_raw(Box::new(LmdbWalletStoreHandle(Arc::clone(
            &(*handle).0.store,
        ))))
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallet_destroy(handle: *mut WalletHandle) {
    drop(Box::from_raw(handle))
}

pub struct RepresentativesLockHandle(MutexGuard<'static, HashSet<PublicKey>>);

#[no_mangle]
pub extern "C" fn rsn_representatives_lock_create(
    handle: &WalletHandle,
) -> *mut RepresentativesLockHandle {
    let guard = handle.0.representatives.lock().unwrap();
    let guard = unsafe {
        std::mem::transmute::<MutexGuard<HashSet<PublicKey>>, MutexGuard<'static, HashSet<PublicKey>>>(
            guard,
        )
    };
    Box::into_raw(Box::new(RepresentativesLockHandle(guard)))
}

#[no_mangle]
pub extern "C" fn rsn_representatives_lock_size(handle: &RepresentativesLockHandle) -> usize {
    handle.0.len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_representatives_lock_insert(
    handle: &mut RepresentativesLockHandle,
    rep: *const u8,
) {
    let rep = PublicKey::from_ptr(rep);
    handle.0.insert(rep);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_representatives_lock_get_all(
    handle: &mut RepresentativesLockHandle,
) -> *mut AccountVecHandle {
    let accounts: Vec<_> = handle.0.iter().map(|i| Account::from(i)).collect();
    Box::into_raw(Box::new(AccountVecHandle(accounts)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_representatives_lock_clear(handle: &mut RepresentativesLockHandle) {
    handle.0.clear();
}

pub struct AccountVecHandle(Vec<Account>);

impl Deref for AccountVecHandle {
    type Target = Vec<Account>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AccountVecHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AccountVecHandle {
    pub fn new(accounts: Vec<Account>) -> *mut AccountVecHandle {
        Box::into_raw(Box::new(Self(accounts)))
    }
}

#[no_mangle]
pub extern "C" fn rsn_account_vec_create() -> *mut AccountVecHandle {
    Box::into_raw(Box::new(AccountVecHandle(Vec::new())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_account_vec_destroy(handle: *mut AccountVecHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub extern "C" fn rsn_account_vec_len(handle: &AccountVecHandle) -> usize {
    handle.0.len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_account_vec_get(
    handle: &AccountVecHandle,
    index: usize,
    result: *mut u8,
) {
    handle.0[index].copy_bytes(result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_account_vec_push(handle: &mut AccountVecHandle, account: *const u8) {
    handle.push(Account::from_ptr(account));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_representatives_lock_destroy(handle: *mut RepresentativesLockHandle) {
    drop(Box::from_raw(handle))
}
