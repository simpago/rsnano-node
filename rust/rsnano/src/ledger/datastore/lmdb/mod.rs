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
mod wallet_store;
mod wallets;

pub use account_store::LmdbAccountStore;
pub use block_store::LmdbBlockStore;
pub use confirmation_height_store::LmdbConfirmationHeightStore;
pub use final_vote_store::LmdbFinalVoteStore;
pub use frontier_store::LmdbFrontierStore;
pub use iterator::LmdbIteratorImpl;
use lmdb::{
    Database, Environment, InactiveTransaction, RoCursor, RoTransaction, RwTransaction, Transaction,
};
pub use lmdb_env::{EnvOptions, LmdbEnv};
#[cfg(test)]
pub(crate) use lmdb_env::{TestDbFile, TestLmdbEnv};
pub use online_weight_store::LmdbOnlineWeightStore;
pub use peer_store::LmdbPeerStore;
pub use pending_store::LmdbPendingStore;
pub use pruned_store::LmdbPrunedStore;
use std::{mem, sync::Arc};
pub use store::{create_backup_file, LmdbStore};
pub use unchecked_store::LmdbUncheckedStore;
pub use version_store::LmdbVersionStore;
pub use wallet_store::LmdbWalletStore;
pub use wallets::LmdbWallets;

use super::{ReadTransaction, TxnCallbacks, WriteTransaction};

enum RoTxnState {
    Inactive(InactiveTransaction<'static>),
    Active(RoTransaction<'static>),
    Transitioning,
}

pub struct LmdbReadTransaction {
    txn_id: u64,
    callbacks: Arc<dyn TxnCallbacks>,
    txn: RoTxnState,
}

impl LmdbReadTransaction {
    pub fn new<'a>(
        txn_id: u64,
        env: &'a Environment,
        callbacks: Arc<dyn TxnCallbacks>,
    ) -> lmdb::Result<Self> {
        let txn = env.begin_ro_txn()?;
        let txn = unsafe { std::mem::transmute::<RoTransaction<'a>, RoTransaction<'static>>(txn) };
        callbacks.txn_start(txn_id, false);

        Ok(Self {
            txn_id,
            callbacks,
            txn: RoTxnState::Active(txn),
        })
    }

    pub fn txn(&self) -> &lmdb::RoTransaction {
        match &self.txn {
            RoTxnState::Active(t) => t,
            _ => panic!("LMDB read transaction not active"),
        }
    }
}

impl Drop for LmdbReadTransaction {
    fn drop(&mut self) {
        let t = mem::replace(&mut self.txn, RoTxnState::Transitioning);
        // This uses commit rather than abort, as it is needed when opening databases with a read only transaction
        match t {
            RoTxnState::Active(t) => t.commit().unwrap(),
            _ => {}
        }
        self.callbacks.txn_end(self.txn_id, false);
    }
}

impl super::Transaction for LmdbReadTransaction {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl ReadTransaction for LmdbReadTransaction {
    fn txn(&self) -> &dyn super::Transaction {
        self
    }

    fn reset(&mut self) {
        let t = mem::replace(&mut self.txn, RoTxnState::Transitioning);
        self.txn = match t {
            RoTxnState::Active(t) => RoTxnState::Inactive(t.reset()),
            RoTxnState::Inactive(_) => panic!("Cannot reset inactive transaction"),
            RoTxnState::Transitioning => unreachable!(),
        };
        self.callbacks.txn_end(self.txn_id, false);
    }

    fn renew(&mut self) {
        let t = mem::replace(&mut self.txn, RoTxnState::Transitioning);
        self.txn = match t {
            RoTxnState::Active(_) => panic!("Cannot renew active transaction"),
            RoTxnState::Inactive(t) => RoTxnState::Active(t.renew().unwrap()),
            RoTxnState::Transitioning => unreachable!(),
        };
        self.callbacks.txn_start(self.txn_id, false);
    }

    fn refresh(&mut self) {
        self.reset();
        self.renew();
    }
}

enum RwTxnState<'a> {
    Inactive(),
    Active(RwTransaction<'a>),
    Transitioning,
}

pub struct LmdbWriteTransaction {
    env: &'static Environment,
    txn_id: u64,
    callbacks: Arc<dyn TxnCallbacks>,
    txn: RwTxnState<'static>,
}

impl LmdbWriteTransaction {
    pub fn new<'a>(
        txn_id: u64,
        env: &'a Environment,
        callbacks: Arc<dyn TxnCallbacks>,
    ) -> lmdb::Result<Self> {
        let env = unsafe { std::mem::transmute::<&'a Environment, &'static Environment>(env) };
        let mut tx = Self {
            env,
            txn_id,
            callbacks,
            txn: RwTxnState::Inactive(),
        };
        tx.renew();
        Ok(tx)
    }

    pub fn rw_txn(&self) -> &RwTransaction<'static> {
        match &self.txn {
            RwTxnState::Active(t) => t,
            _ => panic!("txn not active"),
        }
    }

    pub fn rw_txn_mut(&mut self) -> &mut RwTransaction<'static> {
        match &mut self.txn {
            RwTxnState::Active(t) => t,
            _ => panic!("txn not active"),
        }
    }
}

impl<'a> Drop for LmdbWriteTransaction {
    fn drop(&mut self) {
        self.commit();
    }
}

impl super::Transaction for LmdbWriteTransaction {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl WriteTransaction for LmdbWriteTransaction {
    fn txn(&self) -> &dyn super::Transaction {
        self
    }
    fn txn_mut(&mut self) -> &mut dyn super::Transaction {
        self
    }

    fn renew(&mut self) {
        let t = mem::replace(&mut self.txn, RwTxnState::Transitioning);
        self.txn = match t {
            RwTxnState::Active(_) => panic!("Cannot renew active RwTransaction"),
            RwTxnState::Inactive() => RwTxnState::Active(self.env.begin_rw_txn().unwrap()),
            RwTxnState::Transitioning => unreachable!(),
        };
        self.callbacks.txn_start(self.txn_id, true);
    }

    fn refresh(&mut self) {
        self.commit();
        self.renew();
    }

    fn commit(&mut self) {
        let t = mem::replace(&mut self.txn, RwTxnState::Transitioning);
        match t {
            RwTxnState::Inactive() => {}
            RwTxnState::Active(t) => {
                t.commit().unwrap();
                self.callbacks.txn_end(self.txn_id, true);
            }
            RwTxnState::Transitioning => unreachable!(),
        };
        self.txn = RwTxnState::Inactive();
    }
}

pub(crate) fn exists(txn: &dyn super::Transaction, db: Database, key: &[u8]) -> bool {
    match get(txn, db, &key) {
        Ok(_) => true,
        Err(lmdb::Error::NotFound) => false,
        Err(e) => panic!("exists failed: {:?}", e),
    }
}

pub(crate) fn as_write_txn(txn: &mut dyn WriteTransaction) -> &mut RwTransaction<'static> {
    txn.txn_mut()
        .as_any_mut()
        .downcast_mut::<LmdbWriteTransaction>()
        .unwrap()
        .rw_txn_mut()
}

pub(crate) fn get<'a, K: AsRef<[u8]>>(
    txn: &'a dyn super::Transaction,
    database: Database,
    key: &K,
) -> lmdb::Result<&'a [u8]> {
    let any = txn.as_any();
    if let Some(t) = any.downcast_ref::<LmdbWriteTransaction>() {
        t.rw_txn().get(database, key)
    } else {
        any.downcast_ref::<LmdbReadTransaction>()
            .unwrap()
            .txn()
            .get(database, key)
    }
}

pub(crate) fn open_ro_cursor<'a>(
    txn: &'a dyn super::Transaction,
    database: Database,
) -> lmdb::Result<RoCursor<'a>> {
    let any = txn.as_any();
    if let Some(t) = any.downcast_ref::<LmdbWriteTransaction>() {
        t.rw_txn().open_ro_cursor(database)
    } else {
        any.downcast_ref::<LmdbReadTransaction>()
            .unwrap()
            .txn()
            .open_ro_cursor(database)
    }
}

pub(crate) fn count<'a>(txn: &'a dyn super::Transaction, database: Database) -> usize {
    let any = txn.as_any();
    let stat = if let Some(t) = any.downcast_ref::<LmdbWriteTransaction>() {
        t.rw_txn().stat(database)
    } else {
        any.downcast_ref::<LmdbReadTransaction>()
            .unwrap()
            .txn()
            .stat(database)
    };
    stat.unwrap().entries()
}
