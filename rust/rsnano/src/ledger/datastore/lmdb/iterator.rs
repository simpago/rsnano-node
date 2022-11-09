use std::ffi::c_uint;

use lmdb::{Cursor, Database, RoCursor};
use lmdb_sys::{MDB_FIRST, MDB_LAST, MDB_NEXT, MDB_SET_RANGE};

use crate::{
    ledger::datastore::{iterator::DbIteratorImpl, BinaryDbIterator, DbIterator, Transaction},
    utils::{Deserialize, Serialize},
};

use super::open_ro_cursor;

pub struct LmdbIteratorImpl {
    current: Option<(&'static [u8], &'static [u8])>,
    cursor: Option<RoCursor<'static>>,
}

impl LmdbIteratorImpl {
    pub fn new_iterator<K, V>(
        txn: &dyn Transaction,
        dbi: Database,
        key_val: Option<&[u8]>,
        direction_asc: bool,
    ) -> Box<dyn DbIterator<K, V>>
    where
        K: Serialize + Deserialize<Target = K> + 'static,
        V: Deserialize<Target = V> + 'static,
    {
        let iterator_impl = Self::new(txn, dbi, key_val, direction_asc);
        Box::new(BinaryDbIterator::new(iterator_impl))
    }

    pub fn null_iterator<K, V>() -> Box<dyn DbIterator<K, V>>
    where
        K: Serialize + Deserialize<Target = K> + 'static,
        V: Deserialize<Target = V> + 'static,
    {
        Box::new(BinaryDbIterator::new(Self::null()))
    }

    pub fn new(
        txn: &dyn Transaction,
        dbi: Database,
        key_val: Option<&[u8]>,
        direction_asc: bool,
    ) -> Self {
        let operation = if key_val.is_some() {
            MDB_SET_RANGE
        } else {
            if direction_asc {
                MDB_FIRST
            } else {
                MDB_LAST
            }
        };

        let cursor = open_ro_cursor(txn, dbi).unwrap();
        //todo: dont use unsafe code:
        let cursor =
            unsafe { std::mem::transmute::<lmdb::RoCursor<'_>, lmdb::RoCursor<'static>>(cursor) };
        let mut result = Self {
            current: None,
            cursor: Some(cursor),
        };
        result.load_current(key_val, operation);
        result
    }

    fn load_current(&mut self, key: Option<&[u8]>, operation: c_uint) {
        let result = self.cursor.as_ref().unwrap().get(key, None, operation);
        self.current = match result {
            Err(lmdb::Error::NotFound) => None,
            Ok((Some(k), v)) => Some((k, v)),
            Ok(_) => unreachable!(),
            Err(_) => {
                result.unwrap();
                unreachable!()
            }
        };
    }

    pub fn null() -> Self {
        Self {
            current: None,
            cursor: None,
        }
    }
}

impl DbIteratorImpl for LmdbIteratorImpl {
    fn current(&self) -> Option<(&[u8], &[u8])> {
        self.current
    }

    fn next(&mut self) {
        self.load_current(None, MDB_NEXT);
    }
}

impl PartialEq for LmdbIteratorImpl {
    fn eq(&self, other: &Self) -> bool {
        self.current.map(|(k, _)| k) == other.current.map(|(k, _)| k)
    }
}
