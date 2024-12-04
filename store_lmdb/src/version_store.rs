use crate::{LmdbDatabase, LmdbEnv, LmdbWriteTransaction, Transaction, STORE_VERSION_CURRENT};
use core::panic;
use lmdb::{DatabaseFlags, WriteFlags};
use std::{path::Path, sync::Arc};

pub struct LmdbVersionStore {
    _env: Arc<LmdbEnv>,

    /// U256 (arbitrary key) -> blob
    db_handle: LmdbDatabase,
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
        Ok(Self {
            _env: env,
            db_handle,
        })
    }

    pub fn try_read_version(env: &LmdbEnv) -> Option<i32> {
        match env.environment.open_db(Some("meta")) {
            Ok(db) => {
                let txn = env.tx_begin_read();
                load_version(&txn, db)
            }
            Err(_) => None,
        }
    }

    pub fn check_upgrade(path: &Path) -> anyhow::Result<UpgradeInfo> {
        let env = LmdbEnv::new(path)?;
        let info = match LmdbVersionStore::try_read_version(&env) {
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

    pub fn db_handle(&self) -> LmdbDatabase {
        self.db_handle
    }

    pub fn put(&self, txn: &mut LmdbWriteTransaction, version: i32) {
        let db = self.db_handle();

        let key_bytes = version_key();
        let value_bytes = value_bytes(version);

        txn.put(db, &key_bytes, &value_bytes, WriteFlags::empty())
            .unwrap();
    }

    pub fn get(&self, txn: &dyn Transaction) -> Option<i32> {
        let db = self.db_handle();
        load_version(txn, db)
    }
}

fn load_version(txn: &dyn Transaction, db: LmdbDatabase) -> Option<i32> {
    let key_bytes = version_key();
    match txn.get(db, &key_bytes) {
        Ok(value) => Some(i32::from_be_bytes(value[28..].try_into().unwrap())),
        Err(lmdb::Error::NotFound) => None,
        Err(_) => panic!("Error while loading db version"),
    }
}

fn value_bytes(version: i32) -> [u8; 32] {
    let mut value_bytes = [0; 32];
    value_bytes[28..].copy_from_slice(&version.to_be_bytes());
    value_bytes
}

fn version_key() -> [u8; 32] {
    value_bytes(1)
}
