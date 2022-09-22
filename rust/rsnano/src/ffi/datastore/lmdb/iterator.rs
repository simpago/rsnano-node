use std::{ffi::c_void, ptr};

use crate::{
    datastore::{
        lmdb::{LmdbRawIterator, LmdbReadTransaction, MdbCursor, MdbTxn, MdbVal},
        DbIterator, ReadTransaction,
    },
    ffi::VoidPointerCallback,
};

use super::{TransactionHandle, TransactionType};

pub struct LmdbIteratorHandle(LmdbRawIterator);

impl LmdbIteratorHandle {
    pub fn new(it: LmdbRawIterator) -> *mut Self {
        Box::into_raw(Box::new(LmdbIteratorHandle(it)))
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_iterator_create(
    txn: *mut MdbTxn,
    dbi: u32,
    val: *const MdbVal,
    direction_asc: bool,
    expected_value_size: usize,
) -> *mut LmdbIteratorHandle {
    LmdbIteratorHandle::new(LmdbRawIterator::new(
        txn,
        dbi,
        &*val,
        direction_asc,
        expected_value_size,
    ))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_iterator_destroy(handle: *mut LmdbIteratorHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_iterator_cursor(
    handle: *mut LmdbIteratorHandle,
) -> *mut MdbCursor {
    (*handle).0.cursor()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_iterator_current(
    handle: *mut LmdbIteratorHandle,
    key: *mut MdbVal,
    value: *mut MdbVal,
) {
    *key = (*handle).0.key.clone();
    *value = (*handle).0.value.clone();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_iterator_next(handle: *mut LmdbIteratorHandle) {
    (*handle).0.next();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_iterator_previous(handle: *mut LmdbIteratorHandle) {
    (*handle).0.previous();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_iterator_clear(handle: *mut LmdbIteratorHandle) {
    (*handle).0.clear();
}

pub fn to_lmdb_iterator_handle<K, V>(
    iterator: &mut dyn DbIterator<K, V>,
) -> *mut LmdbIteratorHandle {
    match iterator.take_lmdb_raw_iterator() {
        Some(it) => LmdbIteratorHandle::new(it),
        None => ptr::null_mut(),
    }
}

pub type ForEachParCallback = extern "C" fn(
    *mut c_void,
    *mut TransactionHandle,
    *mut LmdbIteratorHandle,
    *mut LmdbIteratorHandle,
);

pub struct ForEachParWrapper {
    pub action: ForEachParCallback,
    pub context: *mut c_void,
    pub delete_context: VoidPointerCallback,
}

impl ForEachParWrapper {
    pub fn execute<K, V>(
        &self,
        txn: &dyn ReadTransaction,
        begin: &mut dyn DbIterator<K, V>,
        end: &mut dyn DbIterator<K, V>,
    ) {
        let lmdb_txn = txn.as_any().downcast_ref::<LmdbReadTransaction>().unwrap();
        let lmdb_txn = unsafe {
            std::mem::transmute::<&LmdbReadTransaction, &'static LmdbReadTransaction>(lmdb_txn)
        };
        let txn_handle = TransactionHandle::new(TransactionType::ReadRef(lmdb_txn));
        let begin_handle = to_lmdb_iterator_handle(begin);
        let end_handle = to_lmdb_iterator_handle(end);
        (self.action)(self.context, txn_handle, begin_handle, end_handle);
    }
}

unsafe impl Send for ForEachParWrapper {}
unsafe impl Sync for ForEachParWrapper {}

impl Drop for ForEachParWrapper {
    fn drop(&mut self) {
        unsafe { (self.delete_context)(self.context) }
    }
}
