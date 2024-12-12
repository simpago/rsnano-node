use crate::EMPTY_DATABASE;

use super::ConfiguredDatabase;
use lmdb_sys::{MDB_FIRST, MDB_LAST, MDB_NEXT, MDB_PREV, MDB_SET_RANGE};
use std::{cell::Cell, collections::btree_map};

pub struct RoCursor<'txn>(RoCursorStrategy<'txn>);

impl<'txn> RoCursor<'txn> {
    pub fn new_null() -> Self {
        Self::new_null_with(&EMPTY_DATABASE)
    }

    pub fn new_null_with(database: &'txn ConfiguredDatabase) -> Self {
        Self(RoCursorStrategy::Nulled(RoCursorStub {
            database,
            current: Cell::new(0),
        }))
    }

    pub fn new(cursor: lmdb::RoCursor<'txn>) -> Self {
        Self(RoCursorStrategy::Real(cursor))
    }

    pub fn iter_start(&mut self) -> Iter<'txn> {
        match &mut self.0 {
            RoCursorStrategy::Real(s) => Iter::Real(lmdb::Cursor::iter_start(s)),
            RoCursorStrategy::Nulled(s) => s.iter_start(),
        }
    }

    pub fn get(
        &self,
        key: Option<&[u8]>,
        data: Option<&[u8]>,
        op: u32,
    ) -> lmdb::Result<(Option<&'txn [u8]>, &'txn [u8])> {
        match &self.0 {
            RoCursorStrategy::Real(s) => lmdb::Cursor::get(s, key, data, op),
            RoCursorStrategy::Nulled(s) => s.get(key, data, op),
        }
    }
}

enum RoCursorStrategy<'txn> {
    //todo don't use static lifetimes!
    Real(lmdb::RoCursor<'txn>),
    Nulled(RoCursorStub<'txn>),
}

struct RoCursorStub<'txn> {
    database: &'txn ConfiguredDatabase,
    current: Cell<i32>,
}

impl<'txn> RoCursorStub<'txn> {
    fn get(
        &self,
        key: Option<&[u8]>,
        _data: Option<&[u8]>,
        op: u32,
    ) -> lmdb::Result<(Option<&'txn [u8]>, &'txn [u8])> {
        if op == MDB_FIRST {
            self.current.set(0);
        } else if op == MDB_LAST {
            let entry_count = self.database.entries.len();
            self.current.set((entry_count as i32) - 1);
        } else if op == MDB_NEXT {
            self.current.set(self.current.get() + 1);
        } else if op == MDB_PREV {
            self.current.set(self.current.get() - 1);
        } else if op == MDB_SET_RANGE {
            self.current.set(
                self.database
                    .entries
                    .keys()
                    .enumerate()
                    .find_map(|(i, k)| {
                        if Some(k.as_slice()) >= key {
                            Some(i as i32)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(i32::MAX),
            );
        } else {
            unimplemented!()
        }

        let current = self.current.get();
        if current < 0 {
            return Err(lmdb::Error::NotFound);
        }

        self.database
            .entries
            .iter()
            .nth(current as usize)
            .map(|(k, v)| (Some(k.as_slice()), v.as_slice()))
            .ok_or(lmdb::Error::NotFound)
    }

    fn iter_start(&self) -> Iter<'txn> {
        Iter::Stub(self.database.entries.iter())
    }
}

pub enum Iter<'a> {
    Real(lmdb::Iter<'static>),
    Stub(btree_map::Iter<'a, Vec<u8>, Vec<u8>>),
}

impl<'a> Iterator for Iter<'a> {
    type Item = lmdb::Result<(&'static [u8], &'static [u8])>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Iter::Real(i) => i.next(),
            Iter::Stub(iter) => iter.next().map(|(k, v)| unsafe {
                Ok((
                    std::mem::transmute::<&'a [u8], &'static [u8]>(k.as_slice()),
                    std::mem::transmute::<&'a [u8], &'static [u8]>(v.as_slice()),
                ))
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LmdbDatabase, LmdbEnvironment};
    use lmdb::{DatabaseFlags, EnvironmentFlags, Transaction, WriteFlags};
    use std::path::Path;

    #[test]
    fn iter() {
        let _guard1 = FileDropGuard::new("/tmp/rsnano-cursor-test.ldb".as_ref());
        let _guard2 = FileDropGuard::new("/tmp/rsnano-cursor-test.ldb-lock".as_ref());
        let env = create_real_lmdb_env("/tmp/rsnano-cursor-test.ldb");
        create_test_database(&env);
        let env = LmdbEnvironment::new_with(env);
        let database = env.open_db(Some("foo")).unwrap();
        let tx = env.begin_ro_txn().unwrap();
        let mut cursor = tx.open_ro_cursor(database).unwrap();

        let result: Vec<_> = cursor.iter_start().map(|i| i.unwrap()).collect();

        assert_eq!(
            result,
            vec![
                (b"hello".as_ref(), b"world".as_ref()),
                (b"hello2", b"world2"),
                (b"hello3", b"world3")
            ]
        );
    }

    #[test]
    fn iter_backwards() {
        let _guard1 = FileDropGuard::new("/tmp/rsnano-rev-cursor-test.ldb".as_ref());
        let _guard2 = FileDropGuard::new("/tmp/rsnano-rev-cursor-test.ldb-lock".as_ref());
        let env = create_real_lmdb_env("/tmp/rsnano-rev-cursor-test.ldb");
        create_test_database(&env);
        let env = LmdbEnvironment::new_with(env);
        let database = env.open_db(Some("foo")).unwrap();
        let tx = env.begin_ro_txn().unwrap();
        let cursor = tx.open_ro_cursor(database).unwrap();

        assert_eq!(
            cursor.get(None, None, MDB_LAST).unwrap(),
            (Some(b"hello3".as_ref()), b"world3".as_ref())
        );

        assert_eq!(
            cursor.get(None, None, MDB_PREV).unwrap(),
            (Some(b"hello2".as_ref()), b"world2".as_ref())
        );
    }

    mod nullability {
        use super::*;

        const TEST_DATABASE: LmdbDatabase = LmdbDatabase::new_null(42);
        const TEST_DATABASE_NAME: &str = "foo";

        #[test]
        fn iter_from_start() {
            let env = nulled_env_with_foo_database();
            let txn = env.begin_ro_txn().unwrap();
            let mut cursor = txn.open_ro_cursor(TEST_DATABASE).unwrap();

            let result: Vec<([u8; 3], [u8; 3])> = cursor
                .iter_start()
                .map(|i| i.unwrap())
                .map(|(k, v)| (k.try_into().unwrap(), v.try_into().unwrap()))
                .collect();

            assert_eq!(
                result,
                vec![
                    ([1, 1, 1], [6, 6, 6]),
                    ([2, 2, 2], [7, 7, 7]),
                    ([3, 3, 3], [8, 8, 8])
                ]
            )
        }

        #[test]
        fn nulled_cursor_can_be_iterated_forwards() {
            let env = nulled_env_with_foo_database();
            let txn = env.begin_ro_txn().unwrap();

            let cursor = txn.open_ro_cursor(LmdbDatabase::new_null(42)).unwrap();

            let (k, v) = cursor.get(None, None, MDB_FIRST).unwrap();
            assert_eq!(k, Some([1, 1, 1].as_slice()));
            assert_eq!(v, [6, 6, 6].as_slice());

            let (k, v) = cursor.get(None, None, MDB_NEXT).unwrap();
            assert_eq!(k, Some([2, 2, 2].as_slice()));
            assert_eq!(v, [7, 7, 7].as_slice());

            let (k, v) = cursor.get(None, None, MDB_NEXT).unwrap();
            assert_eq!(k, Some([3, 3, 3].as_slice()));
            assert_eq!(v, [8, 8, 8].as_slice());

            let result = cursor.get(None, None, MDB_NEXT);
            assert_eq!(result, Err(lmdb::Error::NotFound));
        }

        #[test]
        fn nulled_cursor_can_be_iterated_backwards() {
            let env = nulled_env_with_foo_database();
            let txn = env.begin_ro_txn().unwrap();
            let cursor = txn.open_ro_cursor(TEST_DATABASE).unwrap();

            let (k, v) = cursor.get(None, None, MDB_LAST).unwrap();
            assert_eq!(k, Some([3, 3, 3].as_slice()));
            assert_eq!(v, [8, 8, 8].as_slice());

            let (k, v) = cursor.get(None, None, MDB_PREV).unwrap();
            assert_eq!(k, Some([2, 2, 2].as_slice()));
            assert_eq!(v, [7, 7, 7].as_slice());

            let (k, v) = cursor.get(None, None, MDB_PREV).unwrap();
            assert_eq!(k, Some([1, 1, 1].as_slice()));
            assert_eq!(v, [6, 6, 6].as_slice());

            let result = cursor.get(None, None, MDB_PREV);
            assert_eq!(result, Err(lmdb::Error::NotFound));
        }

        #[test]
        fn nulled_cursor_can_start_at_specified_key() {
            let env = nulled_env_with_foo_database();
            let txn = env.begin_ro_txn().unwrap();

            let cursor = txn.open_ro_cursor(TEST_DATABASE).unwrap();
            let (k, v) = cursor
                .get(Some([2u8, 2, 2].as_slice()), None, MDB_SET_RANGE)
                .unwrap();
            assert_eq!(k, Some([2, 2, 2].as_slice()));
            assert_eq!(v, [7, 7, 7].as_slice());

            let (k, v) = cursor
                .get(Some([2u8, 1, 0].as_slice()), None, MDB_SET_RANGE)
                .unwrap();
            assert_eq!(k, Some([2, 2, 2].as_slice()));
            assert_eq!(v, [7, 7, 7].as_slice());
        }

        fn nulled_env_with_foo_database() -> LmdbEnvironment {
            LmdbEnvironment::null_builder()
                .database(TEST_DATABASE_NAME, TEST_DATABASE)
                .entry(&[1, 1, 1], &[6, 6, 6])
                .entry(&[2, 2, 2], &[7, 7, 7])
                .entry(&[3, 3, 3], &[8, 8, 8])
                .finish()
                .finish()
        }
    }

    fn create_test_database(env: &lmdb::Environment) {
        env.create_db(Some("foo"), DatabaseFlags::empty()).unwrap();
        let database = env.open_db(Some("foo")).unwrap();
        {
            let mut tx = env.begin_rw_txn().unwrap();
            tx.put(database, b"hello", b"world", WriteFlags::empty())
                .unwrap();
            tx.put(database, b"hello2", b"world2", WriteFlags::empty())
                .unwrap();
            tx.put(database, b"hello3", b"world3", WriteFlags::empty())
                .unwrap();
            tx.commit().unwrap();
        }
    }

    fn create_real_lmdb_env(path: impl AsRef<Path>) -> lmdb::Environment {
        lmdb::Environment::new()
            .set_max_dbs(1)
            .set_map_size(1024 * 1024)
            .set_flags(
                EnvironmentFlags::NO_SUB_DIR
                    | EnvironmentFlags::NO_TLS
                    | EnvironmentFlags::NO_READAHEAD,
            )
            .open(path.as_ref())
            .expect("Could not create LMDB environment")
    }

    struct FileDropGuard<'a> {
        path: &'a Path,
    }

    impl<'a> FileDropGuard<'a> {
        fn new(path: &'a Path) -> Self {
            Self { path }
        }
    }

    impl<'a> Drop for FileDropGuard<'a> {
        fn drop(&mut self) {
            if self.path.exists() {
                let _ = std::fs::remove_file(self.path);
            }
        }
    }
}
