use std::{
    ffi::{c_void, CStr},
    path::{Path, PathBuf},
    ptr,
    sync::Arc,
    time::Duration,
};

use crate::{
    datastore::{
        lmdb::{create_backup_file, EnvOptions, LmdbStore},
        Store,
    },
    ffi::{FfiPropertyTreeWriter, LmdbConfigDto, LoggerHandle, LoggerMT, TxnTrackingConfigDto},
    DiagnosticsConfig, LmdbConfig,
};

use super::{
    account_store::LmdbAccountStoreHandle, block_store::LmdbBlockStoreHandle,
    confirmation_height_store::LmdbConfirmationHeightStoreHandle,
    final_vote_store::LmdbFinalVoteStoreHandle, frontier_store::LmdbFrontierStoreHandle,
    lmdb_env::LmdbEnvHandle, online_weight_store::LmdbOnlineWeightStoreHandle,
    peer_store::LmdbPeerStoreHandle, pending_store::LmdbPendingStoreHandle,
    pruned_store::LmdbPrunedStoreHandle, unchecked_store::LmdbUncheckedStoreHandle,
    version_store::LmdbVersionStoreHandle, TransactionHandle,
};

pub struct LmdbStoreHandle(LmdbStore);

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_create(
    error: *mut bool,
    path: *const i8,
    lmdb_config: *const LmdbConfigDto,
    use_no_mem_init: bool,
    logger: *mut LoggerHandle,
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
    let logger = Arc::new(LoggerMT::new(Box::from_raw(logger)));

    let store = LmdbStore::new(
        path,
        &options,
        txn_config,
        block_processor_batch_max_time,
        logger,
        backup_before_upgrade,
    );
    match store {
        Ok(s) => {
            *error = false;
            Box::into_raw(Box::new(LmdbStoreHandle(s)))
        }
        Err(_) => {
            *error = true;
            eprintln!("Could not create LMDB store");
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_destroy(handle: *mut LmdbStoreHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_env(handle: *mut LmdbStoreHandle) -> *mut LmdbEnvHandle {
    if handle.is_null() {
        ptr::null_mut()
    } else {
        LmdbEnvHandle::new((*handle).0.env.clone())
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_block(
    handle: *mut LmdbStoreHandle,
) -> *mut LmdbBlockStoreHandle {
    if handle.is_null() {
        ptr::null_mut()
    } else {
        LmdbBlockStoreHandle::new((*handle).0.block_store.clone())
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_frontier(
    handle: *mut LmdbStoreHandle,
) -> *mut LmdbFrontierStoreHandle {
    if handle.is_null() {
        ptr::null_mut()
    } else {
        LmdbFrontierStoreHandle::new((*handle).0.frontier_store.clone())
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_account(
    handle: *mut LmdbStoreHandle,
) -> *mut LmdbAccountStoreHandle {
    if handle.is_null() {
        ptr::null_mut()
    } else {
        LmdbAccountStoreHandle::new((*handle).0.account_store.clone())
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_pending(
    handle: *mut LmdbStoreHandle,
) -> *mut LmdbPendingStoreHandle {
    if handle.is_null() {
        ptr::null_mut()
    } else {
        LmdbPendingStoreHandle::new((*handle).0.pending_store.clone())
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_online_weight(
    handle: *mut LmdbStoreHandle,
) -> *mut LmdbOnlineWeightStoreHandle {
    if handle.is_null() {
        ptr::null_mut()
    } else {
        LmdbOnlineWeightStoreHandle::new((*handle).0.online_weight_store.clone())
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_pruned(
    handle: *mut LmdbStoreHandle,
) -> *mut LmdbPrunedStoreHandle {
    if handle.is_null() {
        ptr::null_mut()
    } else {
        LmdbPrunedStoreHandle::new((*handle).0.pruned_store.clone())
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_peer(
    handle: *mut LmdbStoreHandle,
) -> *mut LmdbPeerStoreHandle {
    if handle.is_null() {
        ptr::null_mut()
    } else {
        LmdbPeerStoreHandle::new((*handle).0.peer_store.clone())
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_confirmation_height(
    handle: *mut LmdbStoreHandle,
) -> *mut LmdbConfirmationHeightStoreHandle {
    if handle.is_null() {
        ptr::null_mut()
    } else {
        LmdbConfirmationHeightStoreHandle::new((*handle).0.confirmation_height_store.clone())
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_final_vote(
    handle: *mut LmdbStoreHandle,
) -> *mut LmdbFinalVoteStoreHandle {
    if handle.is_null() {
        ptr::null_mut()
    } else {
        LmdbFinalVoteStoreHandle::new((*handle).0.final_vote_store.clone())
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_unchecked(
    handle: *mut LmdbStoreHandle,
) -> *mut LmdbUncheckedStoreHandle {
    if handle.is_null() {
        ptr::null_mut()
    } else {
        LmdbUncheckedStoreHandle::new((*handle).0.unchecked_store.clone())
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_version(
    handle: *mut LmdbStoreHandle,
) -> *mut LmdbVersionStoreHandle {
    if handle.is_null() {
        ptr::null_mut()
    } else {
        LmdbVersionStoreHandle::new((*handle).0.version_store.clone())
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_store_create_backup_file(
    env: *mut LmdbEnvHandle,
    path: *const i8,
    logger: *mut LoggerHandle,
) -> bool {
    let logger = LoggerMT::new(Box::from_raw(logger));
    let path = CStr::from_ptr(path);
    create_backup_file(&*env, &PathBuf::from(path.to_str().unwrap()), &logger).is_ok()
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
    let mut writer = FfiPropertyTreeWriter::new_borrowed(ptree);
    if let Err(e) = (*handle).0.serialize_memory_stats(&mut writer) {
        eprintln!("memory stat serialization failed: {:?}", e);
    }
}
