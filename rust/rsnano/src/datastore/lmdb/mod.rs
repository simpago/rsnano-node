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
mod txn_tracker;
mod unchecked_store;
mod version_store;

use std::{
    convert::TryFrom,
    ffi::{c_void, CStr, CString},
    os::raw::c_char,
    path::Path,
    ptr,
    sync::Arc,
};

pub use account_store::LmdbAccountStore;
pub use block_store::LmdbBlockStore;
pub use confirmation_height_store::LmdbConfirmationHeightStore;
pub use final_vote_store::LmdbFinalVoteStore;
pub use frontier_store::LmdbFrontierStore;
pub use iterator::{LmdbIterator, LmdbRawIterator};
pub use lmdb_env::{EnvOptions, LmdbEnv};
pub use online_weight_store::LmdbOnlineWeightStore;
pub use peer_store::LmdbPeerStore;
pub use pending_store::LmdbPendingStore;
pub use pruned_store::LmdbPrunedStore;
pub use store::{create_backup_file, LmdbStore, Vacuuming};
pub use txn_tracker::TxnTracker;
pub use unchecked_store::LmdbUncheckedStore;
pub use version_store::LmdbVersionStore;

use crate::{
    utils::{MemoryStream, Serialize, Stream, StreamAdapter},
    Account, BlockHash,
};

use super::{ReadTransaction, Transaction, WriteTransaction};

pub struct LmdbReadTransaction {
    env: *mut MdbEnv,
    txn_id: u64,
    callbacks: Arc<dyn TxnCallbacks>,
    pub handle: *mut MdbTxn,
}

impl LmdbReadTransaction {
    pub unsafe fn new(txn_id: u64, env: *mut MdbEnv, callbacks: Arc<dyn TxnCallbacks>) -> Self {
        let mut handle: *mut MdbTxn = ptr::null_mut();
        let status = mdb_txn_begin(env, ptr::null_mut(), MDB_RDONLY, &mut handle);
        assert!(status == MDB_SUCCESS);
        callbacks.txn_start(txn_id, false);

        Self {
            env,
            txn_id,
            callbacks,
            handle,
        }
    }

    pub fn reset(&mut self) {
        unsafe { mdb_txn_reset(self.handle) };
        self.callbacks.txn_end(self.txn_id, false);
    }

    pub fn renew(&mut self) {
        let status = unsafe { mdb_txn_renew(self.handle) };
        assert!(status == MDB_SUCCESS);
        self.callbacks.txn_start(self.txn_id, false);
    }

    pub fn refresh(&mut self) {
        self.reset();
        self.renew();
    }
}

impl Drop for LmdbReadTransaction {
    fn drop(&mut self) {
        // This uses commit rather than abort, as it is needed when opening databases with a read only transaction
        let status = unsafe { mdb_txn_commit(self.handle) };
        assert!(status == MDB_SUCCESS);
        self.callbacks.txn_end(self.txn_id, false);
    }
}

impl Transaction for LmdbReadTransaction {
    fn as_any(&self) -> &(dyn std::any::Any + '_) {
        self
    }
}

impl ReadTransaction for LmdbReadTransaction {}

pub struct LmdbWriteTransaction {
    env: *mut MdbEnv,
    txn_id: u64,
    callbacks: Arc<dyn TxnCallbacks>,
    pub handle: *mut MdbTxn,
    active: bool,
}

impl LmdbWriteTransaction {
    pub unsafe fn new(txn_id: u64, env: *mut MdbEnv, callbacks: Arc<dyn TxnCallbacks>) -> Self {
        let mut tx = Self {
            env,
            txn_id,
            callbacks,
            handle: ptr::null_mut(),
            active: false,
        };
        tx.renew();
        tx
    }

    pub fn commit(&mut self) {
        if self.active {
            let status = unsafe { mdb_txn_commit(self.handle) };
            self.active = false;
            if status != MDB_SUCCESS {
                let err_msg = unsafe { mdb_strerror(status) };
                panic!(
                    "Unable to write to the LMDB database {}",
                    err_msg.unwrap_or("unknown")
                );
            }
            self.callbacks.txn_end(self.txn_id, true);
        }
    }

    pub fn renew(&mut self) {
        let status = unsafe { mdb_txn_begin(self.env, ptr::null_mut(), 0, &mut self.handle) };
        if status != MDB_SUCCESS {
            let err_msg = unsafe { mdb_strerror(status) };
            panic!("write tx renew failed: {}", err_msg.unwrap_or("unknown"));
        }
        self.active = true;
        self.callbacks.txn_start(self.txn_id, true);
    }

    pub fn refresh(&mut self) {
        self.commit();
        self.renew();
    }
}

impl Drop for LmdbWriteTransaction {
    fn drop(&mut self) {
        self.commit();
    }
}

impl Transaction for LmdbWriteTransaction {
    fn as_any(&self) -> &(dyn std::any::Any + '_) {
        self
    }
}

impl WriteTransaction for LmdbWriteTransaction {
    fn as_transaction(&self) -> &dyn Transaction {
        self
    }
}

pub trait TxnCallbacks {
    fn txn_start(&self, txn_id: u64, is_write: bool);
    fn txn_end(&self, txn_id: u64, is_write: bool);
}

pub struct NullTxnCallbacks {}

impl NullTxnCallbacks {
    pub fn new() -> Self {
        Self {}
    }
}

impl TxnCallbacks for NullTxnCallbacks {
    fn txn_start(&self, _txn_id: u64, _is_write: bool) {}

    fn txn_end(&self, _txn_id: u64, _is_write: bool) {}
}

pub fn assert_success(status: i32) {
    ensure_success(status).unwrap();
}

pub fn ensure_success(status: i32) -> anyhow::Result<()> {
    if status == MDB_SUCCESS {
        Ok(())
    } else {
        let msg = unsafe { mdb_strerror(status) };
        Err(anyhow!(
            "LMDB returned status {}: {}",
            status,
            msg.unwrap_or("unknown")
        ))
    }
}

pub fn exists(txn: &dyn Transaction, table: u32, key: &mut MdbVal) -> bool {
    let mut junk = MdbVal::new();
    let status = unsafe { mdb_get(get_raw_lmdb_txn(txn), table, key, &mut junk) };
    assert!(status == MDB_SUCCESS || status == MDB_NOTFOUND);
    status == MDB_SUCCESS
}

#[repr(C)]
#[derive(PartialEq, Eq)]
pub enum MdbCursorOp {
    MdbFirst,        // Position at first key/data item */
    MdbFirstDup,     // Position at first data item of current key.  Only for #MDB_DUPSORT */
    MdbGetBoth,      // Position at key/data pair. Only for #MDB_DUPSORT */
    MdbGetBothRange, // position at key, nearest data. Only for #MDB_DUPSORT */
    MdbGetCurrent,   // Return key/data at current cursor position */
    MdbGetMultiple, // Return up to a page of duplicate data items from current cursor position. Move cursor to prepare for #MDB_NEXT_MULTIPLE. Only for #MDB_DUPFIXED */
    MdbLast,        // Position at last key/data item */
    MdbLastDup,     // Position at last data item of current key.  Only for #MDB_DUPSORT */
    MdbNext,        // Position at next data item */
    MdbNextDup,     // Position at next data item of current key.  Only for #MDB_DUPSORT */
    MdbNextMultiple, // Return up to a page of duplicate data items from next cursor position. Move cursor to prepare for #MDB_NEXT_MULTIPLE. Only for #MDB_DUPFIXED */
    MdbNextNodup,    // Position at first data item of next key */
    MdbPrev,         // Position at previous data item */
    MdbPrevDup,      // Position at previous data item of current key.  Only for #MDB_DUPSORT */
    MdbPrevNodup,    // Position at last data item of previous key */
    MdbSet,          // Position at specified key */
    MdbSetKey,       // Position at specified key, return key + data */
    MdbSetRange,     // Position at first key greater than or equal to specified key. */
    MdbPrevMultiple, // Position at previous page and return up to a page of duplicate data items. Only for #MDB_DUPFIXED */
}

#[repr(C)]
#[derive(Clone)]
pub struct MdbVal {
    pub mv_size: usize,       // size of the data item
    pub mv_data: *mut c_void, // address of the data item
}

impl MdbVal {
    pub fn new() -> Self {
        Self {
            mv_size: 0,
            mv_data: ptr::null_mut(),
        }
    }

    pub fn from_slice(data: &[u8]) -> Self {
        Self {
            mv_size: data.len(),
            mv_data: data.as_ptr() as *mut c_void,
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.mv_data as *const u8, self.mv_size) }
    }

    pub fn as_stream(&self) -> impl Stream + '_ {
        StreamAdapter::new(self.as_slice())
    }
}

impl Default for MdbVal {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&Account> for MdbVal {
    fn from(a: &Account) -> Self {
        MdbVal::from_slice(a.as_bytes())
    }
}

impl From<&BlockHash> for MdbVal {
    fn from(hash: &BlockHash) -> Self {
        MdbVal::from_slice(hash.as_bytes())
    }
}

impl TryFrom<&MdbVal> for BlockHash {
    type Error = anyhow::Error;

    fn try_from(value: &MdbVal) -> Result<Self, Self::Error> {
        let bytes = value.as_slice().try_into()?;
        Ok(BlockHash::from_bytes(bytes))
    }
}

pub struct OwnedMdbVal {
    bytes: Vec<u8>,
    val: MdbVal,
}

impl OwnedMdbVal {
    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            val: MdbVal {
                mv_size: 0,
                mv_data: ptr::null_mut(),
            },
        }
    }
    pub fn as_mdb_val(&mut self) -> &mut MdbVal {
        self.val.mv_size = self.bytes.len();
        self.val.mv_data = self.bytes.as_mut_ptr() as *mut c_void;
        &mut self.val
    }
}

impl<T> From<&T> for OwnedMdbVal
where
    T: Serialize,
{
    fn from(value: &T) -> Self {
        let mut stream = MemoryStream::new();
        value.serialize(&mut stream).unwrap();
        OwnedMdbVal::new(stream.to_vec())
    }
}

pub fn get_raw_lmdb_txn(txn: &dyn Transaction) -> *mut MdbTxn {
    let any = txn.as_any();
    if let Some(t) = any.downcast_ref::<LmdbReadTransaction>() {
        t.handle
    } else if let Some(t) = any.downcast_ref::<LmdbWriteTransaction>() {
        t.handle
    } else {
        panic!("not an LMDB transaction");
    }
}

/** @brief Statistics for a database in the environment */
#[repr(C)]
#[derive(Default)]
pub struct MdbStat {
    /// Size of a database page.  This is currently the same for all databases.
    pub ms_psize: u32,
    /// Depth (height) of the B-tree
    pub ms_depth: u32,
    /// Number of internal (non-leaf) pages
    pub ms_branch_pages: usize,
    /// Number of leaf pages
    pub ms_leaf_pages: usize,
    /// Number of overflow pages
    pub ms_overflow_pages: usize,
    /// Number of data items
    pub ms_entries: usize,
}

pub unsafe fn mdb_count(txn: *mut MdbTxn, db: u32) -> usize {
    let mut stats = MdbStat::default();
    let status = mdb_stat(txn, db, &mut stats);
    assert_success(status);
    stats.ms_entries
}

#[repr(C)]
pub struct MdbEnv {}

#[repr(C)]
pub struct MdbTxn {}

#[repr(C)]
pub struct MdbCursor {}

pub type MdbTxnBeginCallback =
    extern "C" fn(*mut MdbEnv, *mut MdbTxn, u32, *mut *mut MdbTxn) -> i32;
pub type MdbTxnCommitCallback = extern "C" fn(*mut MdbTxn) -> i32;
pub type MdbTxnResetCallback = extern "C" fn(*mut MdbTxn);
pub type MdbTxnRenewCallback = extern "C" fn(*mut MdbTxn) -> i32;
pub type MdbStrerrorCallback = extern "C" fn(i32) -> *mut c_char;
pub type MdbCursorOpenCallback = extern "C" fn(*mut MdbTxn, u32, *mut *mut MdbCursor) -> i32;
pub type MdbCursorGetCallback =
    extern "C" fn(*mut MdbCursor, *mut MdbVal, *mut MdbVal, MdbCursorOp) -> i32;
pub type MdbCursorCloseCallback = extern "C" fn(*mut MdbCursor);
pub type MdbDbiOpenCallback = extern "C" fn(*mut MdbTxn, *const i8, u32, *mut u32) -> i32;
pub type MdbPutCallback = extern "C" fn(*mut MdbTxn, u32, *mut MdbVal, *mut MdbVal, u32) -> i32;
pub type MdbGetCallback = extern "C" fn(*mut MdbTxn, u32, *mut MdbVal, *mut MdbVal) -> i32;
pub type MdbDelCallback = extern "C" fn(*mut MdbTxn, u32, *mut MdbVal, *mut MdbVal) -> i32;
pub type MdbEnvCreateCallback = extern "C" fn(*mut *mut MdbEnv) -> i32;
pub type MdbEnvSetMaxDbsCallback = extern "C" fn(*mut MdbEnv, u32) -> i32;
pub type MdbEnvSetMapSizeCallback = extern "C" fn(*mut MdbEnv, usize) -> i32;
pub type MdbEnvOpenCallback = extern "C" fn(*mut MdbEnv, *const i8, u32, u32) -> i32;
pub type MdbEnvSyncCallback = extern "C" fn(*mut MdbEnv, i32) -> i32;
pub type MdbEnvCloseCallback = extern "C" fn(*mut MdbEnv);
pub type MdbStatCallback = extern "C" fn(*mut MdbTxn, u32, *mut MdbStat) -> i32;
pub type MdbDropCallback = extern "C" fn(*mut MdbTxn, u32, i32) -> i32;
pub type MdbEnvCopyCallback = extern "C" fn(*mut MdbEnv, *const i8) -> i32;
pub type MdbEnvCopy2Callback = extern "C" fn(*mut MdbEnv, *const i8, u32) -> i32;
pub type MdbEnvStatCallback = extern "C" fn(*mut MdbEnv, *mut MdbStat) -> i32;
pub type MdbDbiCloseCallback = extern "C" fn(*mut MdbEnv, u32);

pub static mut MDB_TXN_BEGIN: Option<MdbTxnBeginCallback> = None;
pub static mut MDB_TXN_COMMIT: Option<MdbTxnCommitCallback> = None;
pub static mut MDB_TXN_RESET: Option<MdbTxnResetCallback> = None;
pub static mut MDB_TXN_RENEW: Option<MdbTxnRenewCallback> = None;
pub static mut MDB_STRERROR: Option<MdbStrerrorCallback> = None;
pub static mut MDB_CURSOR_OPEN: Option<MdbCursorOpenCallback> = None;
pub static mut MDB_CURSOR_GET: Option<MdbCursorGetCallback> = None;
pub static mut MDB_CURSOR_CLOSE: Option<MdbCursorCloseCallback> = None;
pub static mut MDB_DBI_OPEN: Option<MdbDbiOpenCallback> = None;
pub static mut MDB_PUT: Option<MdbPutCallback> = None;
pub static mut MDB_GET: Option<MdbGetCallback> = None;
pub static mut MDB_DEL: Option<MdbDelCallback> = None;
pub static mut MDB_ENV_CREATE: Option<MdbEnvCreateCallback> = None;
pub static mut MDB_ENV_SET_MAX_DBS: Option<MdbEnvSetMaxDbsCallback> = None;
pub static mut MDB_ENV_SET_MAP_SIZE: Option<MdbEnvSetMapSizeCallback> = None;
pub static mut MDB_ENV_OPEN: Option<MdbEnvOpenCallback> = None;
pub static mut MDB_ENV_SYNC: Option<MdbEnvSyncCallback> = None;
pub static mut MDB_ENV_CLOSE: Option<MdbEnvCloseCallback> = None;
pub static mut MDB_STAT: Option<MdbStatCallback> = None;
pub static mut MDB_DROP: Option<MdbDropCallback> = None;
pub static mut MDB_ENV_COPY: Option<MdbEnvCopyCallback> = None;
pub static mut MDB_ENV_COPY2: Option<MdbEnvCopy2Callback> = None;
pub static mut MDB_ENV_STAT: Option<MdbEnvStatCallback> = None;
pub static mut MDB_DBI_CLOSE: Option<MdbDbiCloseCallback> = None;

pub unsafe fn mdb_txn_begin(
    env: *mut MdbEnv,
    parent: *mut MdbTxn,
    flags: u32,
    result: *mut *mut MdbTxn,
) -> i32 {
    MDB_TXN_BEGIN.expect("MDB_TXN_BEGIN missing")(env, parent, flags, result)
}

pub unsafe fn mdb_txn_commit(txn: *mut MdbTxn) -> i32 {
    MDB_TXN_COMMIT.expect("MDB_TXN_COMMIT missing")(txn)
}

pub unsafe fn mdb_txn_reset(txn: *mut MdbTxn) {
    MDB_TXN_RESET.expect("MDB_TXN_RESET missing")(txn)
}

pub unsafe fn mdb_txn_renew(txn: *mut MdbTxn) -> i32 {
    MDB_TXN_RENEW.expect("MDB_TXN_RENEW missing")(txn)
}

pub unsafe fn mdb_strerror(status: i32) -> Option<&'static str> {
    let ptr = MDB_STRERROR.expect("MDB_STRERROR missing")(status);
    if ptr.is_null() {
        None
    } else {
        Some(CStr::from_ptr(ptr).to_str().unwrap())
    }
}

pub unsafe fn mdb_cursor_open(txn: *mut MdbTxn, dbi: u32, cursor: *mut *mut MdbCursor) -> i32 {
    MDB_CURSOR_OPEN.expect("MDB_CURSOR_OPEN missing")(txn, dbi, cursor)
}

pub unsafe fn mdb_cursor_get(
    cursor: *mut MdbCursor,
    key: &mut MdbVal,
    value: &mut MdbVal,
    op: MdbCursorOp,
) -> i32 {
    MDB_CURSOR_GET.expect("MDB_CURSOR_GET missing")(cursor, key, value, op)
}

pub unsafe fn mdb_cursor_close(cursor: *mut MdbCursor) {
    MDB_CURSOR_CLOSE.expect("MDB_CURSOR_CLOSE missing")(cursor);
}

pub unsafe fn mdb_dbi_open(txn: *mut MdbTxn, name: &str, flags: u32, dbi: &mut u32) -> i32 {
    let name_cstr = CString::new(name).unwrap();
    MDB_DBI_OPEN.expect("MDB_DBI_OPEN missing")(txn, name_cstr.as_ptr(), flags, dbi)
}

pub unsafe fn mdb_put(
    txn: *mut MdbTxn,
    dbi: u32,
    key: &mut MdbVal,
    data: &mut MdbVal,
    flags: u32,
) -> i32 {
    MDB_PUT.expect("MDB_PUT missing")(txn, dbi, key, data, flags)
}

pub unsafe fn mdb_get(txn: *mut MdbTxn, dbi: u32, key: &mut MdbVal, data: &mut MdbVal) -> i32 {
    MDB_GET.expect("MDB_GET missing")(txn, dbi, key, data)
}

pub unsafe fn mdb_del(
    txn: *mut MdbTxn,
    dbi: u32,
    key: &mut MdbVal,
    data: Option<&mut MdbVal>,
) -> i32 {
    let dataptr = data.map(|v| v as *mut MdbVal).unwrap_or(ptr::null_mut());
    MDB_DEL.expect("MDB_DEL missing")(txn, dbi, key, dataptr)
}

pub unsafe fn mdb_env_create(env: *mut *mut MdbEnv) -> i32 {
    MDB_ENV_CREATE.expect("MDB_ENV_CREATE missing")(env)
}

pub unsafe fn mdb_env_set_maxdbs(env: *mut MdbEnv, max_dbs: u32) -> i32 {
    MDB_ENV_SET_MAX_DBS.expect("MDB_ENV_SET_MAX_DBS missing")(env, max_dbs)
}

pub unsafe fn mdb_env_set_mapsize(env: *mut MdbEnv, size: usize) -> i32 {
    MDB_ENV_SET_MAP_SIZE.expect("MDB_ENV_SET_MAP_SIZE missing")(env, size)
}

pub unsafe fn mdb_env_open(env: *mut MdbEnv, path: &Path, flags: u32, mode: u32) -> i32 {
    let path_cstr = CString::new(path.to_str().unwrap()).unwrap();
    MDB_ENV_OPEN.expect("MDB_ENV_OPEN missing")(env, path_cstr.as_ptr(), flags, mode)
}

pub unsafe fn mdb_env_sync(env: *mut MdbEnv, force: bool) -> i32 {
    MDB_ENV_SYNC.expect("MDB_ENV_SYNC missing")(env, force as i32)
}

pub unsafe fn mdb_env_close(env: *mut MdbEnv) {
    MDB_ENV_CLOSE.expect("MDB_ENV_CLOSE missing")(env);
}

pub unsafe fn mdb_stat(txn: *mut MdbTxn, dbi: u32, stat: &mut MdbStat) -> i32 {
    MDB_STAT.expect("MDB_STAT missing")(txn, dbi, stat)
}

pub unsafe fn mdb_drop(txn: *mut MdbTxn, dbi: u32, del: i32) -> i32 {
    MDB_DROP.expect("MDB_DROP missing")(txn, dbi, del)
}

pub unsafe fn mdb_env_copy(env: *mut MdbEnv, target: *const i8) -> i32 {
    MDB_ENV_COPY.expect("MDB_ENV_COPY missing")(env, target)
}

pub unsafe fn mdb_env_copy2(env: *mut MdbEnv, path: *const i8, flags: u32) -> i32 {
    MDB_ENV_COPY2.expect("MDB_ENV_COPY2 missing")(env, path, flags)
}

pub unsafe fn mdb_env_stat(env: *mut MdbEnv, stat: *mut MdbStat) -> i32 {
    MDB_ENV_STAT.expect("MDB_ENV_STAT missing")(env, stat)
}

pub unsafe fn mdb_dbi_close(env: *mut MdbEnv, dbi: u32) {
    MDB_DBI_CLOSE.expect("MDB_DBI_CLOSE missing")(env, dbi);
}

/// Successful result
const MDB_SUCCESS: i32 = 0;

/// key/data pair not found (EOF)
const MDB_NOTFOUND: i32 = -30798;

/// create DB if not already existing */
const MDB_CREATE: u32 = 0x40000;

/// Data is being appended, don't split full pages. */
const MDB_APPEND: u32 = 0x20000;

/// Compacting copy: Omit free space from copy, and renumber all
/// pages sequentially.
const MDB_CP_COMPACT: u32 = 0x01;

// mdb_env environment flags:

/// mmap at a fixed address (experimental)
const MDB_FIXEDMAP: u32 = 0x01;
/// no environment directory
const MDB_NOSUBDIR: u32 = 0x4000;
/// don't fsync after commit
const MDB_NOSYNC: u32 = 0x10000;
/// read only
const MDB_RDONLY: u32 = 0x20000;
/// don't fsync metapage after commit
const MDB_NOMETASYNC: u32 = 0x40000;
/// use writable mmap
const MDB_WRITEMAP: u32 = 0x80000;
/// use asynchronous msync when #WRITEMAP is used
const MDB_MAPASYNC: u32 = 0x100000;
/// tie reader locktable slots to #txn objects instead of to threads
const MDB_NOTLS: u32 = 0x200000;
/// don't do any locking, caller must manage their own locks
const MDB_NOLOCK: u32 = 0x400000;
/// don't do readahead (no effect on Windows)
const MDB_NORDAHEAD: u32 = 0x800000;
/// don't initialize malloc'd memory before writing to datafile
const MDB_NOMEMINIT: u32 = 0x1000000;
