use lmdb_sys::{MDB_cursor_op, MDB_FIRST, MDB_LAST, MDB_NEXT, MDB_PREV, MDB_SET_RANGE};
use rsnano_core::utils::{BufferReader, Deserialize, MutStreamAdapter, Serialize};
use rsnano_nullable_lmdb::{RoCursor, EMPTY_DATABASE};
use std::{
    cmp::Ordering,
    marker::PhantomData,
    ops::{Bound, RangeBounds},
};

pub struct LmdbRangeIterator<'txn, K, V, R> {
    cursor: RoCursor<'txn>,
    range: R,
    initialized: bool,
    empty: bool,
    phantom: PhantomData<(K, V)>,
}

impl<'txn, K, V, R> LmdbRangeIterator<'txn, K, V, R>
where
    K: Deserialize<Target = K> + Serialize + Ord,
    V: Deserialize<Target = V>,
    R: RangeBounds<K>,
{
    pub fn new(cursor: RoCursor<'txn>, range: R) -> Self {
        Self {
            cursor,
            range,
            initialized: false,
            empty: false,
            phantom: Default::default(),
        }
    }

    pub fn empty(range: R) -> Self {
        Self {
            cursor: RoCursor::new_null_with(&EMPTY_DATABASE),
            range,
            initialized: false,
            empty: true,
            phantom: Default::default(),
        }
    }

    fn get_next_result(&mut self) -> lmdb::Result<(Option<&'txn [u8]>, &'txn [u8])> {
        if self.empty {
            Err(lmdb::Error::NotFound)
        } else if !self.initialized {
            self.initialized = true;
            self.get_first_result()
        } else {
            self.cursor.get(None, None, MDB_NEXT)
        }
    }

    fn get_first_result(&self) -> lmdb::Result<(Option<&'txn [u8]>, &'txn [u8])> {
        match self.range.start_bound() {
            Bound::Included(start) => {
                let mut key_bytes = [0u8; 64];
                let mut stream = MutStreamAdapter::new(&mut key_bytes);
                start.serialize(&mut stream);
                self.cursor.get(Some(stream.written()), None, MDB_SET_RANGE)
            }
            Bound::Excluded(_) => unimplemented!(),
            Bound::Unbounded => self.cursor.get(None, None, MDB_FIRST),
        }
    }

    fn deserialize(&self, key_bytes: Option<&[u8]>, value_bytes: &[u8]) -> (K, V) {
        let mut stream = BufferReader::new(key_bytes.unwrap());
        let key = K::deserialize(&mut stream).unwrap();
        let mut stream = BufferReader::new(value_bytes);
        let value = V::deserialize(&mut stream).unwrap();
        (key, value)
    }

    fn should_include(&self, key: &K) -> bool {
        match self.range.end_bound() {
            Bound::Included(end) => {
                matches!(key.cmp(end), Ordering::Less | Ordering::Equal)
            }
            Bound::Excluded(end) => matches!(key.cmp(end), Ordering::Less),
            Bound::Unbounded => true,
        }
    }
}

impl<'txn, K, V, R> Iterator for LmdbRangeIterator<'txn, K, V, R>
where
    K: Deserialize<Target = K> + Serialize + Ord,
    V: Deserialize<Target = V>,
    R: RangeBounds<K>,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        match self.get_next_result() {
            Ok((key, value)) => {
                let result = self.deserialize(key, value);
                if self.should_include(&result.0) {
                    Some(result)
                } else {
                    None
                }
            }
            Err(lmdb::Error::NotFound) => None,
            Err(e) => panic!("Could not read from cursor: {:?}", e),
        }
    }
}

pub struct LmdbIterator<'txn, K, V>
where
    K: Serialize,
{
    cursor: RoCursor<'txn>,
    operation: MDB_cursor_op,
    next_op: MDB_cursor_op,
    convert: fn(&[u8], &[u8]) -> (K, V),
}

impl<'txn, K, V> LmdbIterator<'txn, K, V>
where
    K: Serialize,
{
    pub fn new(cursor: RoCursor<'txn>, convert: fn(&[u8], &[u8]) -> (K, V)) -> Self {
        Self {
            cursor,
            operation: MDB_FIRST,
            next_op: MDB_NEXT,
            convert,
        }
    }

    pub fn new_descending(cursor: RoCursor<'txn>, convert: fn(&[u8], &[u8]) -> (K, V)) -> Self {
        Self {
            cursor,
            operation: MDB_LAST,
            next_op: MDB_PREV,
            convert,
        }
    }
}

impl<'txn, K, V> Iterator for LmdbIterator<'txn, K, V>
where
    K: Serialize,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        let result = match self.cursor.get(None, None, self.operation) {
            Err(lmdb::Error::NotFound) => None,
            Ok((Some(k), v)) => Some((self.convert)(k, v)),
            Ok(_) => panic!("No key returned"),
            Err(e) => panic!("Read error {:?}", e),
        };
        self.operation = self.next_op;
        result
    }
}
