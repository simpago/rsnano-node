use rsnano_core::Account;
use rsnano_node::consensus::RepTiers;
use std::{ops::Deref, sync::Arc};

pub struct RepTiersHandle(pub Arc<RepTiers>);

impl Deref for RepTiersHandle {
    type Target = Arc<RepTiers>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_rep_tiers_destroy(handle: *mut RepTiersHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_rep_tiers_tier(
    handle: &RepTiersHandle,
    representative: *const u8,
) -> u8 {
    handle.tier(&Account::from_ptr(representative)) as u8
}
