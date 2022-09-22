use super::{mdb_cursor_close, mdb_cursor_open, MdbCursor, MdbTxn, MdbVal, OwnedMdbVal};
use crate::{
    datastore::{
        lmdb::{get_raw_lmdb_txn, mdb_cursor_get, MdbCursorOp, MDB_NOTFOUND, MDB_SUCCESS},
        DbIterator, Transaction,
    },
    utils::{Deserialize, Serialize, StreamAdapter},
};
use std::ptr;

#[derive(Clone)]
pub struct LmdbRawIterator {
    cursor: *mut MdbCursor,
    pub key: MdbVal,
    pub value: MdbVal,
    expected_key_size: usize,
}

impl LmdbRawIterator {
    pub fn new(
        txn: *mut MdbTxn,
        dbi: u32,
        val: &MdbVal,
        direction_asc: bool,
        expected_key_size: usize,
    ) -> Self {
        let mut iterator = Self {
            cursor: ptr::null_mut(),
            key: MdbVal::new(),
            value: MdbVal::new(),
            expected_key_size,
        };
        iterator.init(txn, dbi, val, direction_asc);
        iterator
    }

    pub fn take(&mut self) -> Self {
        let result = self.clone();
        self.cursor = ptr::null_mut();
        result
    }

    fn init(&mut self, txn: *mut MdbTxn, dbi: u32, val_a: &MdbVal, direction_asc: bool) {
        let status = unsafe { mdb_cursor_open(txn, dbi, &mut self.cursor) };
        assert!(status == MDB_SUCCESS);

        let mut operation = MdbCursorOp::MdbSetRange;
        if val_a.mv_size != 0 {
            self.key = val_a.clone();
        } else {
            operation = if direction_asc {
                MdbCursorOp::MdbFirst
            } else {
                MdbCursorOp::MdbLast
            };
        }
        let status2 =
            unsafe { mdb_cursor_get(self.cursor, &mut self.key, &mut self.value, operation) };
        assert!(status2 == MDB_SUCCESS || status2 == MDB_NOTFOUND);
        if status2 != MDB_NOTFOUND {
            let status3 = unsafe {
                mdb_cursor_get(
                    self.cursor,
                    &mut self.key,
                    &mut self.value,
                    MdbCursorOp::MdbGetCurrent,
                )
            };
            assert!(status3 == MDB_SUCCESS || status3 == MDB_NOTFOUND);
            if self.key.mv_size != self.expected_key_size {
                self.clear();
            }
        } else {
            self.clear();
        }
    }

    pub fn clear(&mut self) {
        self.key = MdbVal::new();
        self.value = MdbVal::new();
    }

    pub fn cursor(&self) -> *mut MdbCursor {
        self.cursor
    }

    pub fn set_cursor(&mut self, cursor: *mut MdbCursor) {
        self.cursor = cursor;
    }

    pub fn next(&mut self) {
        debug_assert!(!self.cursor.is_null());
        let status = unsafe {
            mdb_cursor_get(
                self.cursor,
                &mut self.key,
                &mut self.value,
                MdbCursorOp::MdbNext,
            )
        };
        assert!(status == MDB_SUCCESS || status == MDB_NOTFOUND);
        if status == MDB_NOTFOUND {
            self.clear();
        }
        if self.key.mv_size != self.expected_key_size {
            self.clear();
        }
    }

    pub fn previous(&mut self) {
        debug_assert!(!self.cursor.is_null());
        let status = unsafe {
            mdb_cursor_get(
                self.cursor,
                &mut self.key,
                &mut self.value,
                MdbCursorOp::MdbPrev,
            )
        };
        assert!(status == MDB_SUCCESS || status == MDB_NOTFOUND);
        if status == MDB_NOTFOUND {
            self.clear();
        }
        if self.key.mv_size != self.expected_key_size {
            self.clear();
        }
    }
}

impl Drop for LmdbRawIterator {
    fn drop(&mut self) {
        if !self.cursor.is_null() {
            unsafe { mdb_cursor_close(self.cursor) };
        }
    }
}

pub struct LmdbIterator<K, V>
where
    K: Serialize + Deserialize<Target = K>,
    V: Deserialize<Target = V>,
{
    key: Option<K>,
    value: Option<V>,
    raw_iterator: LmdbRawIterator,
}

impl<K, V> LmdbIterator<K, V>
where
    K: Serialize + Deserialize<Target = K>,
    V: Deserialize<Target = V>,
{
    pub fn new(txn: &dyn Transaction, dbi: u32, key: Option<&K>, direction_asc: bool) -> Self {
        let mut key_val = match key {
            Some(key) => OwnedMdbVal::from(key),
            None => OwnedMdbVal::empty(),
        };
        let raw_iterator = LmdbRawIterator::new(
            get_raw_lmdb_txn(txn),
            dbi,
            key_val.as_mdb_val(),
            direction_asc,
            K::serialized_size(),
        );
        let mut result = Self {
            key: None,
            value: None,
            raw_iterator,
        };
        result.load_current();
        result
    }

    pub fn as_raw(self) -> LmdbRawIterator {
        self.raw_iterator
    }

    fn load_current(&mut self) {
        self.key = if self.raw_iterator.key.mv_size > 0 {
            Some(K::deserialize(&mut StreamAdapter::new(self.raw_iterator.key.as_slice())).unwrap())
        } else {
            None
        };

        self.value = if self.key.is_some() {
            Some(
                V::deserialize(&mut StreamAdapter::new(self.raw_iterator.value.as_slice()))
                    .unwrap(),
            )
        } else {
            None
        }
    }
}

impl<K, V> DbIterator<K, V> for LmdbIterator<K, V>
where
    K: Serialize + Deserialize<Target = K>,
    V: Deserialize<Target = V>,
{
    fn take_lmdb_raw_iterator(&mut self) -> Option<LmdbRawIterator> {
        Some(self.raw_iterator.take())
    }

    fn is_end(&self) -> bool {
        self.raw_iterator.key.mv_size == 0
    }

    fn value(&self) -> Option<&V> {
        self.value.as_ref()
    }

    fn current(&self) -> Option<(&K, &V)> {
        if let Some(k) = self.key.as_ref() {
            Some((k, self.value.as_ref().unwrap()))
        } else {
            None
        }
    }

    fn next(&mut self) {
        self.raw_iterator.next();
        self.load_current();
    }
}
