use crate::{LmdbDatabase, Transaction};
use lmdb_sys::{MDB_cursor_op, MDB_FIRST, MDB_LAST, MDB_NEXT, MDB_SET_RANGE};
use rsnano_core::utils::{
    BufferReader, Deserialize, FixedSizeSerialize, MutStreamAdapter, Serialize,
};
use rsnano_nullable_lmdb::RoCursor;
use std::{
    cmp::Ordering,
    ffi::c_uint,
    marker::PhantomData,
    ops::{Bound, RangeBounds},
};

pub struct BinaryDbIterator<'txn, K, V>
where
    K: FixedSizeSerialize + Deserialize<Target = K>,
    V: Deserialize<Target = V>,
{
    iterator_impl: Option<LmdbIteratorImpl<'txn>>,
    current: Option<(K, V)>,
}

impl<'txn, K, V> PartialEq for BinaryDbIterator<'txn, K, V>
where
    K: FixedSizeSerialize + Deserialize<Target = K>,
    V: Deserialize<Target = V>,
{
    fn eq(&self, other: &Self) -> bool {
        self.iterator_impl == other.iterator_impl
    }
}

impl<'txn, K, V> BinaryDbIterator<'txn, K, V>
where
    K: FixedSizeSerialize + Deserialize<Target = K>,
    V: Deserialize<Target = V>,
{
    pub fn new(iterator_impl: LmdbIteratorImpl<'txn>) -> Self {
        let mut result = Self {
            iterator_impl: Some(iterator_impl),
            current: None,
        };
        result.load_current();
        result
    }

    fn load_current(&mut self) {
        self.current = match self.iterator_impl.as_ref().unwrap().current() {
            Some((k, v)) => {
                if k.len() < K::serialized_size() {
                    None
                } else {
                    let key = K::deserialize(&mut BufferReader::new(k)).unwrap();
                    let value = V::deserialize(&mut BufferReader::new(v)).unwrap();
                    Some((key, value))
                }
            }
            None => None,
        };
    }

    pub fn take_impl(&mut self) -> LmdbIteratorImpl<'txn> {
        self.iterator_impl.take().unwrap()
    }

    pub fn is_end(&self) -> bool {
        self.iterator_impl.as_ref().unwrap().current().is_none()
    }

    pub fn current(&self) -> Option<(&K, &V)> {
        self.current.as_ref().map(|(k, v)| (k, v))
    }

    pub fn next(&mut self) {
        self.iterator_impl.as_mut().unwrap().next();
        self.load_current();
    }

    pub fn eq(&self, other: &BinaryDbIterator<K, V>) -> bool {
        self.iterator_impl.eq(&other.iterator_impl)
    }
}

pub struct LmdbIteratorImpl<'txn> {
    current: Option<(&'txn [u8], &'txn [u8])>,
    cursor: Option<RoCursor<'txn>>,
}

impl<'txn> LmdbIteratorImpl<'txn> {
    pub fn new_iterator<K, V>(
        txn: &'txn dyn Transaction,
        dbi: LmdbDatabase,
        key_val: Option<&[u8]>,
        direction_asc: bool,
    ) -> BinaryDbIterator<'txn, K, V>
    where
        K: FixedSizeSerialize + Deserialize<Target = K> + 'static,
        V: Deserialize<Target = V> + 'static,
    {
        let iterator_impl = Self::new(txn, dbi, key_val, direction_asc);
        BinaryDbIterator::new(iterator_impl)
    }

    pub fn null_iterator<K, V>() -> BinaryDbIterator<'txn, K, V>
    where
        K: FixedSizeSerialize + Deserialize<Target = K> + 'static,
        V: Deserialize<Target = V> + 'static,
    {
        BinaryDbIterator::new(Self::null())
    }

    pub fn new(
        txn: &'txn dyn Transaction,
        dbi: LmdbDatabase,
        key_val: Option<&[u8]>,
        direction_asc: bool,
    ) -> LmdbIteratorImpl<'txn> {
        let operation = if key_val.is_some() {
            MDB_SET_RANGE
        } else if direction_asc {
            MDB_FIRST
        } else {
            MDB_LAST
        };

        let cursor = txn.open_ro_cursor(dbi).unwrap();
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

    pub fn null() -> LmdbIteratorImpl<'static> {
        LmdbIteratorImpl::<'static> {
            current: None,
            cursor: None,
        }
    }

    pub fn current(&self) -> Option<(&[u8], &[u8])> {
        self.current
    }

    pub fn next(&mut self) {
        self.load_current(None, MDB_NEXT);
    }
}

impl<'txn> PartialEq for LmdbIteratorImpl<'txn> {
    fn eq(&self, other: &Self) -> bool {
        self.current.map(|(k, _)| k) == other.current.map(|(k, _)| k)
    }
}

pub struct LmdbRangeIterator<'txn, K, V, R> {
    cursor: RoCursor<'txn>,
    range: R,
    initialized: bool,
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
            phantom: Default::default(),
        }
    }

    fn get_next_result(&mut self) -> lmdb::Result<(Option<&'txn [u8]>, &'txn [u8])> {
        if !self.initialized {
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
            convert,
        }
    }

    pub fn start_at(&mut self, k: &K) -> Option<(K, V)> {
        self.operation = MDB_NEXT;
        let mut buffer = [0; 64];
        let mut key_buffer = MutStreamAdapter::new(&mut buffer);
        k.serialize(&mut key_buffer);
        self.read(MDB_SET_RANGE, Some(key_buffer.written()))
    }

    fn read(&self, operation: MDB_cursor_op, key: Option<&[u8]>) -> Option<(K, V)> {
        match self.cursor.get(key, None, operation) {
            Err(lmdb::Error::NotFound) => None,
            Ok((Some(k), v)) => Some((self.convert)(k, v)),
            Ok(_) => panic!("No key returned"),
            Err(e) => panic!("Read error {:?}", e),
        }
    }
}

impl<'txn, K, V> Iterator for LmdbIterator<'txn, K, V>
where
    K: Serialize,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.read(self.operation, None);
        self.operation = MDB_NEXT;
        result
    }
}
