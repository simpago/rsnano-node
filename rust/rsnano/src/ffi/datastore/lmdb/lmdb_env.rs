use std::{
    ffi::{c_void, CStr},
    ops::Deref,
    path::Path,
    sync::Arc,
    time::Duration,
};

use crate::{
    datastore::lmdb::{EnvOptions, LmdbEnv},
    ffi::{FfiPropertyTreeWriter, LmdbConfigDto, LoggerHandle, LoggerMT, TxnTrackingConfigDto},
    DiagnosticsConfig, LmdbConfig,
};

use super::{TransactionHandle, TransactionType};

pub struct LmdbEnvHandle(Arc<LmdbEnv>);

impl LmdbEnvHandle {
    pub fn new(env: Arc<LmdbEnv>) -> *mut Self {
        Box::into_raw(Box::new(LmdbEnvHandle(env)))
    }
}

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
    let env = LmdbEnv::new(&mut *error, path, &options);
    if *error {
        eprintln!("Could not create LMDB env");
    }
    Box::into_raw(Box::new(LmdbEnvHandle(Arc::new(env))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_mdb_env_create2(
    error: *mut bool,
    path: *const i8,
    lmdb_config: *const LmdbConfigDto,
    use_no_mem_init: bool,
    logger: *mut LoggerHandle,
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
    let txn_config = DiagnosticsConfig::from(&*txn_config).txn_tracking;
    let block_processor_batch_max_time = Duration::from_millis(block_processor_batch_max_time_ms);
    let logger = Arc::new(LoggerMT::new(Box::from_raw(logger)));
    let env = LmdbEnv::with_tracking(
        path,
        &options,
        txn_config,
        block_processor_batch_max_time,
        logger,
    );
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
pub unsafe extern "C" fn rsn_mdb_env_init(
    handle: *mut LmdbEnvHandle,
    error: *mut bool,
    path: *const i8,
    lmdb_config: *const LmdbConfigDto,
    use_no_mem_init: bool,
) {
    let config = LmdbConfig::from(&*lmdb_config);
    let options = EnvOptions {
        config,
        use_no_mem_init,
    };
    let path_str = CStr::from_ptr(path).to_str().unwrap();
    let path = Path::new(path_str);
    *error = (*handle).init(path, &options).is_err();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_mdb_env_close(handle: *mut LmdbEnvHandle) {
    (*handle).close();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_mdb_env_destroy(handle: *mut LmdbEnvHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_mdb_env_get_env(handle: *mut LmdbEnvHandle) -> *mut c_void {
    (*handle).0.env() as *mut c_void
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
    let mut ptree = FfiPropertyTreeWriter::new_borrowed(ptree);
    (*handle)
        .0
        .serialize_txn_tracker(
            &mut ptree,
            Duration::from_millis(min_read_time_ms),
            Duration::from_millis(min_write_time_ms),
        )
        .unwrap()
}
