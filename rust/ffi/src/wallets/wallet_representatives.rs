use rsnano_core::{Account, Amount, PublicKey};
use rsnano_node::wallets::WalletRepresentatives;
use std::{
    ops::{Deref, DerefMut},
    sync::MutexGuard,
};

pub struct WalletRepresentativesLock(MutexGuard<'static, WalletRepresentatives>);

impl WalletRepresentativesLock {
    pub unsafe fn new(guard: MutexGuard<WalletRepresentatives>) -> *mut WalletRepresentativesLock {
        let guard = std::mem::transmute::<
            MutexGuard<WalletRepresentatives>,
            MutexGuard<'static, WalletRepresentatives>,
        >(guard);
        Box::into_raw(Box::new(WalletRepresentativesLock(guard)))
    }
}

impl Deref for WalletRepresentativesLock {
    type Target = WalletRepresentatives;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for WalletRepresentativesLock {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallet_representatives_lock_destroy(
    handle: *mut WalletRepresentativesLock,
) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub extern "C" fn rsn_wallet_representatives_lock_have_half_rep(
    handle: &WalletRepresentativesLock,
) -> bool {
    handle.have_half_rep()
}

#[no_mangle]
pub extern "C" fn rsn_wallet_representatives_lock_voting_reps(
    handle: &WalletRepresentativesLock,
) -> u64 {
    handle.voting_reps()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallet_representatives_lock_exists(
    handle: &WalletRepresentativesLock,
    rep: *const u8,
) -> bool {
    handle.exists(&Account::from_ptr(rep))
}

#[no_mangle]
pub extern "C" fn rsn_wallet_representatives_lock_clear(handle: &mut WalletRepresentativesLock) {
    handle.clear();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_wallet_representatives_lock_check_rep(
    handle: &mut WalletRepresentativesLock,
    account: *const u8,
    half_principal_weight: *const u8,
) -> bool {
    handle.check_rep(
        PublicKey::from_ptr(account),
        Amount::from_ptr(half_principal_weight),
    )
}
