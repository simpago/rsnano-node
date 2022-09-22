mod account_store;
mod block_store;
mod confirmation_height_store;
mod final_vote_store;
mod frontier_store;
mod iterator;
mod lmdb_env;
mod online_weight_store;
mod peer_store;
mod pending_store;
mod pruned_store;
mod store;
mod unchecked_store;
mod version_store;

use std::{ffi::c_void, ops::Deref, sync::Arc};

use crate::{
    datastore::{
        lmdb::{
            LmdbReadTransaction, LmdbWriteTransaction, MdbCursorCloseCallback,
            MdbCursorGetCallback, MdbCursorOpenCallback, MdbDbiCloseCallback, MdbDbiOpenCallback,
            MdbDelCallback, MdbDropCallback, MdbEnv, MdbEnvCloseCallback, MdbEnvCopy2Callback,
            MdbEnvCopyCallback, MdbEnvCreateCallback, MdbEnvOpenCallback, MdbEnvSetMapSizeCallback,
            MdbEnvSetMaxDbsCallback, MdbEnvStatCallback, MdbEnvSyncCallback, MdbGetCallback,
            MdbPutCallback, MdbStatCallback, MdbStrerrorCallback, MdbTxn, MdbTxnBeginCallback,
            MdbTxnCommitCallback, MdbTxnRenewCallback, MdbTxnResetCallback, TxnCallbacks,
            MDB_CURSOR_CLOSE, MDB_CURSOR_GET, MDB_CURSOR_OPEN, MDB_DBI_CLOSE, MDB_DBI_OPEN,
            MDB_DEL, MDB_DROP, MDB_ENV_CLOSE, MDB_ENV_COPY, MDB_ENV_COPY2, MDB_ENV_CREATE,
            MDB_ENV_OPEN, MDB_ENV_SET_MAP_SIZE, MDB_ENV_SET_MAX_DBS, MDB_ENV_STAT, MDB_ENV_SYNC,
            MDB_GET, MDB_PUT, MDB_STAT, MDB_STRERROR, MDB_TXN_BEGIN, MDB_TXN_COMMIT, MDB_TXN_RENEW,
            MDB_TXN_RESET,
        },
        Transaction,
    },
    ffi::VoidPointerCallback,
};

pub struct TransactionHandle(TransactionType);

impl TransactionHandle {
    pub fn new(txn_type: TransactionType) -> *mut TransactionHandle {
        Box::into_raw(Box::new(TransactionHandle(txn_type)))
    }

    pub fn as_read_txn_mut(&mut self) -> &mut LmdbReadTransaction {
        match &mut self.0 {
            TransactionType::Read(tx) => tx,
            _ => panic!("invalid tx type"),
        }
    }

    pub fn as_read_txn(&mut self) -> &LmdbReadTransaction {
        match &mut self.0 {
            TransactionType::Read(tx) => tx,
            TransactionType::ReadRef(tx) => *tx,
            _ => panic!("invalid tx type"),
        }
    }

    pub fn as_write_txn(&mut self) -> &mut LmdbWriteTransaction {
        match &mut self.0 {
            TransactionType::Write(tx) => tx,
            _ => panic!("invalid tx type"),
        }
    }

    pub fn as_txn(&self) -> &dyn Transaction {
        match &self.0 {
            TransactionType::Read(t) => t,
            TransactionType::ReadRef(t) => *t,
            TransactionType::Write(t) => t,
        }
    }
}

impl Deref for TransactionHandle {
    type Target = TransactionType;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub enum TransactionType {
    Read(LmdbReadTransaction),
    ReadRef(&'static LmdbReadTransaction),
    Write(LmdbWriteTransaction),
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_read_txn_create(
    txn_id: u64,
    env: *mut MdbEnv,
    callbacks: *mut c_void,
) -> *mut TransactionHandle {
    let callbacks = Arc::new(FfiCallbacksWrapper::new(callbacks));
    Box::into_raw(Box::new(TransactionHandle(TransactionType::Read(
        LmdbReadTransaction::new(txn_id, env, callbacks),
    ))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_read_txn_destroy(handle: *mut TransactionHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_read_txn_reset(handle: *mut TransactionHandle) {
    (*handle).as_read_txn_mut().reset();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_read_txn_renew(handle: *mut TransactionHandle) {
    (*handle).as_read_txn_mut().renew();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_read_txn_refresh(handle: *mut TransactionHandle) {
    (*handle).as_read_txn_mut().refresh();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_read_txn_handle(handle: *mut TransactionHandle) -> *mut MdbTxn {
    (*handle).as_read_txn().handle
}

#[no_mangle]
pub extern "C" fn rsn_lmdb_write_txn_create(
    txn_id: u64,
    env: *mut MdbEnv,
    callbacks: *mut c_void,
) -> *mut TransactionHandle {
    let callbacks = Arc::new(FfiCallbacksWrapper::new(callbacks));
    Box::into_raw(Box::new(TransactionHandle(TransactionType::Write(
        unsafe { LmdbWriteTransaction::new(txn_id, env, callbacks) },
    ))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_write_txn_destroy(handle: *mut TransactionHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_write_txn_commit(handle: *mut TransactionHandle) {
    (*handle).as_write_txn().commit();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_write_txn_renew(handle: *mut TransactionHandle) {
    (*handle).as_write_txn().renew();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_write_txn_refresh(handle: *mut TransactionHandle) {
    (*handle).as_write_txn().refresh();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_write_txn_handle(handle: *mut TransactionHandle) -> *mut MdbTxn {
    (*handle).as_write_txn().handle
}

struct FfiCallbacksWrapper {
    handle: *mut c_void,
}

impl FfiCallbacksWrapper {
    fn new(handle: *mut c_void) -> Self {
        Self { handle }
    }
}

impl TxnCallbacks for FfiCallbacksWrapper {
    fn txn_start(&self, txn_id: u64, is_write: bool) {
        unsafe { TXN_START.expect("TXN_START")(self.handle, txn_id, is_write) }
    }

    fn txn_end(&self, txn_id: u64, _is_write: bool) {
        unsafe { TXN_END.expect("TXN_END")(self.handle, txn_id) }
    }
}

impl Drop for FfiCallbacksWrapper {
    fn drop(&mut self) {
        unsafe { TXN_CALLBACKS_DESTROY.expect("TXN_CALLBACKS_DESTROY missing")(self.handle) }
    }
}

static mut TXN_CALLBACKS_DESTROY: Option<VoidPointerCallback> = None;
pub type TxnStartCallback = unsafe extern "C" fn(*mut c_void, u64, bool);
pub type TxnEndCallback = unsafe extern "C" fn(*mut c_void, u64);
static mut TXN_START: Option<TxnStartCallback> = None;
static mut TXN_END: Option<TxnEndCallback> = None;

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_txn_callbacks_destroy(f: VoidPointerCallback) {
    TXN_CALLBACKS_DESTROY = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_txn_callbacks_start(f: TxnStartCallback) {
    TXN_START = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_txn_callbacks_end(f: TxnEndCallback) {
    TXN_END = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_txn_begin(f: MdbTxnBeginCallback) {
    MDB_TXN_BEGIN = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_txn_commit(f: MdbTxnCommitCallback) {
    MDB_TXN_COMMIT = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_txn_reset(f: MdbTxnResetCallback) {
    MDB_TXN_RESET = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_txn_renew(f: MdbTxnRenewCallback) {
    MDB_TXN_RENEW = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_strerror(f: MdbStrerrorCallback) {
    MDB_STRERROR = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_cursor_open(f: MdbCursorOpenCallback) {
    MDB_CURSOR_OPEN = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_cursor_get(f: MdbCursorGetCallback) {
    MDB_CURSOR_GET = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_cursor_close(f: MdbCursorCloseCallback) {
    MDB_CURSOR_CLOSE = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_dbi_open(f: MdbDbiOpenCallback) {
    MDB_DBI_OPEN = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_put(f: MdbPutCallback) {
    MDB_PUT = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_get(f: MdbGetCallback) {
    MDB_GET = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_del(f: MdbDelCallback) {
    MDB_DEL = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_env_create(f: MdbEnvCreateCallback) {
    MDB_ENV_CREATE = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_env_set_map_size(f: MdbEnvSetMapSizeCallback) {
    MDB_ENV_SET_MAP_SIZE = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_env_set_max_dbs(f: MdbEnvSetMaxDbsCallback) {
    MDB_ENV_SET_MAX_DBS = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_env_open(f: MdbEnvOpenCallback) {
    MDB_ENV_OPEN = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_env_sync(f: MdbEnvSyncCallback) {
    MDB_ENV_SYNC = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_env_close(f: MdbEnvCloseCallback) {
    MDB_ENV_CLOSE = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_stat(f: MdbStatCallback) {
    MDB_STAT = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_drop(f: MdbDropCallback) {
    MDB_DROP = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_env_copy(f: MdbEnvCopyCallback) {
    MDB_ENV_COPY = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_env_copy2(f: MdbEnvCopy2Callback) {
    MDB_ENV_COPY2 = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_env_stat(f: MdbEnvStatCallback) {
    MDB_ENV_STAT = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_mdb_dbi_close(f: MdbDbiCloseCallback) {
    MDB_DBI_CLOSE = Some(f);
}
