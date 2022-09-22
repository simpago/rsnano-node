use std::sync::{Arc, Mutex};

use crate::{
    datastore::{
        lmdb::{assert_success, mdb_put, MDB_NOTFOUND, MDB_SUCCESS},
        parallel_traversal_u512, DbIterator, FinalVoteStore, NullIterator, Transaction,
        WriteTransaction,
    },
    BlockHash, QualifiedRoot, Root,
};

use super::{
    ensure_success, get_raw_lmdb_txn, mdb_count, mdb_dbi_open, mdb_del, mdb_drop, mdb_get, LmdbEnv,
    LmdbIterator, MdbVal,
};

/// Maps root to block hash for generated final votes.
/// nano::qualified_root -> nano::block_hash
pub struct LmdbFinalVoteStore {
    env: Arc<LmdbEnv>,
    db_handle: Mutex<u32>,
}

impl LmdbFinalVoteStore {
    pub fn new(env: Arc<LmdbEnv>) -> Self {
        Self {
            env,
            db_handle: Mutex::new(0),
        }
    }

    pub fn db_handle(&self) -> u32 {
        *self.db_handle.lock().unwrap()
    }

    pub fn open_db(&self, txn: &dyn Transaction, flags: u32) -> anyhow::Result<()> {
        let mut handle = 0;
        let status =
            unsafe { mdb_dbi_open(get_raw_lmdb_txn(txn), "final_votes", flags, &mut handle) };
        *self.db_handle.lock().unwrap() = handle;
        ensure_success(status)
    }
}

impl FinalVoteStore for LmdbFinalVoteStore {
    fn put(&self, txn: &dyn WriteTransaction, root: &QualifiedRoot, hash: &BlockHash) -> bool {
        let mut value = MdbVal::new();
        let root_bytes = root.to_bytes();
        let status = unsafe {
            mdb_get(
                get_raw_lmdb_txn(txn.as_transaction()),
                self.db_handle(),
                &mut MdbVal::from_slice(&root_bytes),
                &mut value,
            )
        };
        assert!(status == MDB_SUCCESS || status == MDB_NOTFOUND);
        if status == MDB_SUCCESS {
            BlockHash::try_from(&value).unwrap() == *hash
        } else {
            let status = unsafe {
                mdb_put(
                    get_raw_lmdb_txn(txn.as_transaction()),
                    self.db_handle(),
                    &mut MdbVal::from_slice(&root_bytes),
                    &mut MdbVal::from(hash),
                    0,
                )
            };
            assert_success(status);
            true
        }
    }

    fn begin(&self, txn: &dyn Transaction) -> Box<dyn DbIterator<QualifiedRoot, BlockHash>> {
        Box::new(LmdbIterator::new(txn, self.db_handle(), None, true))
    }

    fn begin_at_root(
        &self,
        txn: &dyn Transaction,
        root: &QualifiedRoot,
    ) -> Box<dyn DbIterator<QualifiedRoot, BlockHash>> {
        Box::new(LmdbIterator::new(txn, self.db_handle(), Some(root), true))
    }

    fn get(&self, txn: &dyn Transaction, root: Root) -> Vec<BlockHash> {
        let mut result = Vec::new();
        let key_start = QualifiedRoot {
            root,
            previous: BlockHash::new(),
        };

        let mut i = self.begin_at_root(txn, &key_start);
        while let Some((k, v)) = i.current() {
            if k.root != root {
                break;
            }

            result.push(*v);
            i.next();
        }

        result
    }

    fn del(&self, txn: &dyn WriteTransaction, root: Root) {
        let mut final_vote_qualified_roots = Vec::new();

        let mut it = self.begin_at_root(
            txn.as_transaction(),
            &QualifiedRoot {
                root,
                previous: BlockHash::new(),
            },
        );
        while let Some((k, _)) = it.current() {
            if k.root != root {
                break;
            }
            final_vote_qualified_roots.push(k.clone());
            it.next();
        }

        for qualified_root in final_vote_qualified_roots {
            let root_bytes = qualified_root.to_bytes();
            let status = unsafe {
                mdb_del(
                    get_raw_lmdb_txn(txn.as_transaction()),
                    self.db_handle(),
                    &mut MdbVal::from_slice(&root_bytes),
                    None,
                )
            };
            assert_success(status);
        }
    }

    fn count(&self, txn: &dyn Transaction) -> usize {
        unsafe { mdb_count(get_raw_lmdb_txn(txn), self.db_handle()) }
    }

    fn clear(&self, txn: &dyn WriteTransaction) {
        unsafe {
            mdb_drop(get_raw_lmdb_txn(txn.as_transaction()), self.db_handle(), 0);
        }
    }

    fn for_each_par(
        &self,
        action: &(dyn Fn(
            &dyn crate::datastore::ReadTransaction,
            &mut dyn DbIterator<QualifiedRoot, BlockHash>,
            &mut dyn DbIterator<QualifiedRoot, BlockHash>,
        ) + Send
              + Sync),
    ) {
        parallel_traversal_u512(&|start, end, is_last| {
            let mut transaction = self.env.tx_begin_read();
            let mut begin_it = self.begin_at_root(&transaction, &start.into());
            let mut end_it = if !is_last {
                self.begin_at_root(&transaction, &end.into())
            } else {
                self.end()
            };
            action(&mut transaction, begin_it.as_mut(), end_it.as_mut());
        });
    }

    fn end(&self) -> Box<dyn DbIterator<QualifiedRoot, BlockHash>> {
        Box::new(NullIterator::new())
    }
}
