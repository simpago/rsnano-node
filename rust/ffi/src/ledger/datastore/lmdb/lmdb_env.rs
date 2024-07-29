use super::{TransactionHandle, TransactionType};
use crate::{FfiPropertyTree, LmdbConfigDto, TxnTrackingConfigDto};
use rsnano_node::utils::{LongRunningTransactionLogger, TxnTrackingConfig};
use rsnano_store_lmdb::{
    EnvOptions, LmdbConfig, LmdbEnv, NullTransactionTracker, TransactionTracker,
};
use std::{
    ffi::{c_void, CStr},
    ops::Deref,
    path::Path,
    sync::Arc,
    time::Duration,
};

pub struct LmdbEnvHandle(Arc<LmdbEnv>);

impl Deref for LmdbEnvHandle {
    type Target = Arc<LmdbEnv>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_mdb_env_create(
    error: *mut bool,
    path: *const i8,
    lmdb_config: *const LmdbConfigDto,
    use_no_mem_init: bool,
) -> *mut LmdbEnvHandle {
    let config = LmdbConfig::from(&*lmdb_config);
    let options = EnvOptions {
        config,
        use_no_mem_init,
    };
    let path_str = CStr::from_ptr(path).to_str().unwrap();
    let path = Path::new(path_str);
    match LmdbEnv::new_with_options(path, &options) {
        Ok(env) => {
            *error = false;
            Box::into_raw(Box::new(LmdbEnvHandle(Arc::new(env))))
        }
        Err(_) => {
            eprintln!("Could not create LMDB env");
            *error = true;
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_mdb_env_create2(
    error: *mut bool,
    path: *const i8,
    lmdb_config: *const LmdbConfigDto,
    use_no_mem_init: bool,
    txn_config: *const TxnTrackingConfigDto,
    block_processor_batch_max_time_ms: u64,
) -> *mut LmdbEnvHandle {
    let config = LmdbConfig::from(&*lmdb_config);
    let options = EnvOptions {
        config,
        use_no_mem_init,
    };
    let path_str = CStr::from_ptr(path).to_str().unwrap();
    let path = Path::new(path_str);
    let txn_config: TxnTrackingConfig = (&*txn_config).into();
    let block_processor_batch_max_time = Duration::from_millis(block_processor_batch_max_time_ms);

    let txn_tracker: Arc<dyn TransactionTracker> = if txn_config.enable {
        Arc::new(LongRunningTransactionLogger::new(
            txn_config,
            block_processor_batch_max_time,
        ))
    } else {
        Arc::new(NullTransactionTracker::new())
    };

    let env = LmdbEnv::new_with_txn_tracker(path, &options, txn_tracker);
    match env {
        Ok(e) => {
            *error = false;
            Box::into_raw(Box::new(LmdbEnvHandle(Arc::new(e))))
        }
        Err(_) => {
            *error = true;
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_mdb_env_destroy(handle: *mut LmdbEnvHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_mdb_env_tx_begin_read(
    handle: *mut LmdbEnvHandle,
) -> *mut TransactionHandle {
    let txn = (*handle).0.tx_begin_read();
    TransactionHandle::new(TransactionType::Read(txn))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_mdb_env_tx_begin_write(
    handle: *mut LmdbEnvHandle,
) -> *mut TransactionHandle {
    let txn = (*handle).0.tx_begin_write();
    TransactionHandle::new(TransactionType::Write(txn))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_mdb_env_serialize_txn_tracker(
    handle: *mut LmdbEnvHandle,
    ptree: *mut c_void,
    min_read_time_ms: u64,
    min_write_time_ms: u64,
) {
    let mut ptree = FfiPropertyTree::new_borrowed(ptree);
    (*handle)
        .0
        .serialize_txn_tracker(
            &mut ptree,
            Duration::from_millis(min_read_time_ms),
            Duration::from_millis(min_write_time_ms),
        )
        .unwrap()
}
