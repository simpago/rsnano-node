use super::{
    account_store::LmdbAccountStoreHandle, block_store::LmdbBlockStoreHandle,
    confirmation_height_store::LmdbConfirmationHeightStoreHandle,
    final_vote_store::LmdbFinalVoteStoreHandle, online_weight_store::LmdbOnlineWeightStoreHandle,
    peer_store::LmdbPeerStoreHandle, pending_store::LmdbPendingStoreHandle,
    pruned_store::LmdbPrunedStoreHandle, version_store::LmdbVersionStoreHandle, TransactionHandle,
    TransactionType,
};
use crate::{FfiPropertyTree, LmdbConfigDto, StringDto, TxnTrackingConfigDto};
use rsnano_node::{config::DiagnosticsConfig, utils::LongRunningTransactionLogger};
use rsnano_store_lmdb::{
    EnvOptions, LmdbConfig, LmdbStore, NullTransactionTracker, TransactionTracker,
};
use std::{
    ffi::{c_void, CStr},
    ops::Deref,
    path::{Path, PathBuf},
    ptr,
    sync::Arc,
    time::Duration,
};

pub struct LmdbStoreHandle(pub Arc<LmdbStore>);

impl Deref for LmdbStoreHandle {
    type Target = Arc<LmdbStore>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_create_v2(
    error: *mut bool,
    path: *const i8,
    lmdb_config: *const LmdbConfigDto,
    use_no_mem_init: bool,
    txn_config: *const TxnTrackingConfigDto,
    block_processor_batch_max_time_ms: u64,
    backup_before_upgrade: bool,
) -> *mut LmdbStoreHandle {
    let config = LmdbConfig::from(&*lmdb_config);
    let options = EnvOptions {
        config,
        use_no_mem_init,
    };
    let path_str = CStr::from_ptr(path).to_str().unwrap();
    let path = Path::new(path_str);
    let txn_config = DiagnosticsConfig::from(&*txn_config).txn_tracking;
    let block_processor_batch_max_time = Duration::from_millis(block_processor_batch_max_time_ms);

    let txn_tracker: Arc<dyn TransactionTracker> = if txn_config.enable {
        Arc::new(LongRunningTransactionLogger::new(
            txn_config,
            block_processor_batch_max_time,
        ))
    } else {
        Arc::new(NullTransactionTracker::new())
    };

    let store = LmdbStore::open(path)
        .options(&options)
        .txn_tracker(txn_tracker)
        .backup_before_upgrade(backup_before_upgrade)
        .build();

    match store {
        Ok(s) => {
            *error = false;
            Box::into_raw(Box::new(LmdbStoreHandle(Arc::new(s))))
        }
        Err(e) => {
            *error = true;
            eprintln!(
                "Could not create LMDB store: {:?}. LMDB options: {:?}",
                e, options
            );
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_destroy(handle: *mut LmdbStoreHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_block(
    handle: *mut LmdbStoreHandle,
) -> *mut LmdbBlockStoreHandle {
    if handle.is_null() {
        ptr::null_mut()
    } else {
        LmdbBlockStoreHandle::new((*handle).0.block.clone())
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_account(
    handle: *mut LmdbStoreHandle,
) -> *mut LmdbAccountStoreHandle {
    if handle.is_null() {
        ptr::null_mut()
    } else {
        LmdbAccountStoreHandle::new((*handle).0.account.clone())
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_pending(
    handle: *mut LmdbStoreHandle,
) -> *mut LmdbPendingStoreHandle {
    if handle.is_null() {
        ptr::null_mut()
    } else {
        LmdbPendingStoreHandle::new((*handle).0.pending.clone())
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_online_weight(
    handle: *mut LmdbStoreHandle,
) -> *mut LmdbOnlineWeightStoreHandle {
    if handle.is_null() {
        ptr::null_mut()
    } else {
        LmdbOnlineWeightStoreHandle::new((*handle).0.online_weight.clone())
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_pruned(
    handle: *mut LmdbStoreHandle,
) -> *mut LmdbPrunedStoreHandle {
    if handle.is_null() {
        ptr::null_mut()
    } else {
        LmdbPrunedStoreHandle::new((*handle).0.pruned.clone())
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_peer(
    handle: *mut LmdbStoreHandle,
) -> *mut LmdbPeerStoreHandle {
    if handle.is_null() {
        ptr::null_mut()
    } else {
        LmdbPeerStoreHandle::new((*handle).0.peer.clone())
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_confirmation_height(
    handle: *mut LmdbStoreHandle,
) -> *mut LmdbConfirmationHeightStoreHandle {
    if handle.is_null() {
        ptr::null_mut()
    } else {
        LmdbConfirmationHeightStoreHandle::new((*handle).0.confirmation_height.clone())
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_final_vote(
    handle: *mut LmdbStoreHandle,
) -> *mut LmdbFinalVoteStoreHandle {
    if handle.is_null() {
        ptr::null_mut()
    } else {
        LmdbFinalVoteStoreHandle::new((*handle).0.final_vote.clone())
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_version(
    handle: *mut LmdbStoreHandle,
) -> *mut LmdbVersionStoreHandle {
    if handle.is_null() {
        ptr::null_mut()
    } else {
        LmdbVersionStoreHandle::new((*handle).0.version.clone())
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_copy_db(
    handle: *mut LmdbStoreHandle,
    path: *const i8,
) -> bool {
    let path = PathBuf::from(CStr::from_ptr(path).to_str().unwrap());
    (*handle).0.copy_db(&path).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_rebuild_db(
    handle: *mut LmdbStoreHandle,
    txn: *mut TransactionHandle,
) {
    if let Err(e) = (*handle).0.rebuild_db((*txn).as_write_txn()) {
        eprintln!("rebuild db failed: {:?}", e);
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_serialize_memory_stats(
    handle: *mut LmdbStoreHandle,
    ptree: *mut c_void,
) {
    let mut writer = FfiPropertyTree::new_borrowed(ptree);
    if let Err(e) = (*handle).0.serialize_memory_stats(&mut writer) {
        eprintln!("memory stat serialization failed: {:?}", e);
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_vendor_get(
    handle: *mut LmdbStoreHandle,
    result: *mut StringDto,
) {
    *result = (*handle).0.vendor().into()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_serialize_mdb_tracker(
    handle: *mut LmdbStoreHandle,
    ptree: *mut c_void,
    min_read_time_ms: u64,
    min_write_time_ms: u64,
) {
    let mut ptree = FfiPropertyTree::new_borrowed(ptree);
    (*handle)
        .0
        .serialize_mdb_tracker(
            &mut ptree,
            Duration::from_millis(min_read_time_ms),
            Duration::from_millis(min_write_time_ms),
        )
        .unwrap()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_tx_begin_read(
    handle: *mut LmdbStoreHandle,
) -> *mut TransactionHandle {
    let txn = (*handle).0.tx_begin_read();
    TransactionHandle::new(TransactionType::Read(txn))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_tx_begin_write(
    handle: *mut LmdbStoreHandle,
) -> *mut TransactionHandle {
    let txn = (*handle).0.tx_begin_write();
    TransactionHandle::new(TransactionType::Write(txn))
}
