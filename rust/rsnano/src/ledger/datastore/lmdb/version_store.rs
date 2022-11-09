use super::{as_write_txn, get, LmdbEnv};
use crate::ledger::datastore::{
    Transaction, VersionStore, WriteTransaction, STORE_VERSION_CURRENT, STORE_VERSION_MINIMUM,
};
use lmdb::{Database, DatabaseFlags, WriteFlags};
use std::{path::Path, sync::Arc};

pub struct LmdbVersionStore {
    env: Arc<LmdbEnv>,

    /// U256 (arbitrary key) -> blob
    db_handle: Database,
}

pub struct UpgradeInfo {
    pub is_fresh_db: bool,
    pub is_fully_upgraded: bool,
}

impl LmdbVersionStore {
    pub fn new(env: Arc<LmdbEnv>) -> anyhow::Result<Self> {
        let db_handle = env
            .environment
            .create_db(Some("meta"), DatabaseFlags::empty())?;
        Ok(Self { env, db_handle })
    }

    pub fn try_read_version(env: &LmdbEnv) -> anyhow::Result<Option<i32>> {
        match env.environment.open_db(Some("meta")) {
            Ok(db) => {
                let txn = env.tx_begin_read()?;
                Ok(Some(load_version(&txn, db)))
            }
            Err(_) => Ok(None),
        }
    }

    pub fn check_upgrade(path: &Path) -> anyhow::Result<UpgradeInfo> {
        let env = LmdbEnv::new(path)?;
        let info = match LmdbVersionStore::try_read_version(&env)? {
            Some(version) => UpgradeInfo {
                is_fresh_db: false,
                is_fully_upgraded: version == STORE_VERSION_CURRENT,
            },
            None => UpgradeInfo {
                is_fresh_db: true,
                is_fully_upgraded: false,
            },
        };
        Ok(info)
    }

    pub fn db_handle(&self) -> Database {
        self.db_handle
    }
}

impl VersionStore for LmdbVersionStore {
    fn put(&self, txn: &mut dyn WriteTransaction, version: i32) {
        let db = self.db_handle();

        let key_bytes = version_key();
        let value_bytes = value_bytes(version);

        as_write_txn(txn)
            .put(db, &key_bytes, &value_bytes, WriteFlags::empty())
            .unwrap();
    }

    fn get(&self, txn: &dyn Transaction) -> i32 {
        let db = self.db_handle();
        load_version(txn, db)
    }
}

fn load_version(txn: &dyn Transaction, db: Database) -> i32 {
    let key_bytes = version_key();
    match get(txn, db, &key_bytes) {
        Ok(value) => i32::from_ne_bytes(value[28..].try_into().unwrap()),
        Err(_) => STORE_VERSION_MINIMUM,
    }
}

fn value_bytes(version: i32) -> [u8; 32] {
    let mut value_bytes = [0; 32];
    value_bytes[28..].copy_from_slice(&version.to_ne_bytes());
    value_bytes
}

fn version_key() -> [u8; 32] {
    value_bytes(1)
}
