use crate::datastore::Ledger;
use std::{ffi::c_void, ops::Deref, sync::Arc};

pub struct LedgerHandle(Arc<Ledger>);

impl Deref for LedgerHandle {
    type Target = Arc<Ledger>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub extern "C" fn rsn_ledger_create(handle: *mut c_void) -> *mut LedgerHandle {
    let ledger = Ledger::new(handle);
    Box::into_raw(Box::new(LedgerHandle(Arc::new(ledger))))
}

#[no_mangle]
pub extern "C" fn rsn_ledger_destroy(handle: *mut LedgerHandle) {
    drop(unsafe { Box::from_raw(handle) });
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_ledger_block_or_pruned_exists(
    f: LedgerBlockOrPrunedExistsCallback,
) {
    BLOCK_OR_PRUNED_EXISTS_CALLBACK = Some(f);
}

type LedgerBlockOrPrunedExistsCallback = unsafe extern "C" fn(*mut c_void, *const u8) -> bool;
pub(crate) static mut BLOCK_OR_PRUNED_EXISTS_CALLBACK: Option<LedgerBlockOrPrunedExistsCallback> =
    None;
