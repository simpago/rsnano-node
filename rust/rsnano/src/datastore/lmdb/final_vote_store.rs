use std::sync::Arc;

use crate::{
    datastore::{
        lmdb::{assert_success, mdb_put, MDB_NOTFOUND, MDB_SUCCESS},
        DbIterator, FinalVoteStore, Transaction, WriteTransaction,
    },
    BlockHash, QualifiedRoot, Root,
};

use super::{get_raw_lmdb_txn, mdb_del, mdb_get, LmdbEnv, LmdbIterator, MdbVal};

/// Maps root to block hash for generated final votes.
/// nano::qualified_root -> nano::block_hash
pub struct LmdbFinalVoteStore {
    env: Arc<LmdbEnv>,
    pub table_handle: u32,
}

impl LmdbFinalVoteStore {
    pub fn new(env: Arc<LmdbEnv>) -> Self {
        Self {
            env,
            table_handle: 0,
        }
    }
}

impl FinalVoteStore for LmdbFinalVoteStore {
    fn put(&self, txn: &dyn WriteTransaction, root: &QualifiedRoot, hash: &BlockHash) -> bool {
        let mut value = MdbVal::new();
        let root_bytes = root.to_bytes();
        let status = unsafe {
            mdb_get(
                get_raw_lmdb_txn(txn.as_transaction()),
                self.table_handle,
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
                    self.table_handle,
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
        Box::new(LmdbIterator::new(txn, self.table_handle, None, true))
    }

    fn begin_at_root(
        &self,
        txn: &dyn Transaction,
        root: &QualifiedRoot,
    ) -> Box<dyn DbIterator<QualifiedRoot, BlockHash>> {
        Box::new(LmdbIterator::new(txn, self.table_handle, Some(root), true))
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
                    self.table_handle,
                    &mut MdbVal::from_slice(&root_bytes),
                    None,
                )
            };
            assert_success(status);
        }
    }
}
