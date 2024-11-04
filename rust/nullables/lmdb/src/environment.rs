use crate::ConfiguredDatabaseBuilder;

use super::{ConfiguredDatabase, LmdbDatabase, RoTransaction, RwTransaction};
use lmdb::{DatabaseFlags, EnvironmentFlags, Stat};
use lmdb_sys::MDB_env;
use std::path::Path;

pub struct EnvironmentOptions<'a> {
    pub max_dbs: u32,
    pub map_size: usize,
    pub flags: EnvironmentFlags,
    pub path: &'a Path,
    pub file_mode: u32,
}

pub struct LmdbEnvironment(EnvironmentStrategy);

impl LmdbEnvironment {
    pub fn new(options: EnvironmentOptions) -> lmdb::Result<Self> {
        Ok(Self(EnvironmentStrategy::Real(EnvironmentWrapper::build(
            options,
        )?)))
    }

    pub fn new_with(env: lmdb::Environment) -> Self {
        Self(EnvironmentStrategy::Real(EnvironmentWrapper::new(env)))
    }

    pub fn new_null() -> Self {
        Self(EnvironmentStrategy::Nulled(EnvironmentStub {
            databases: Vec::new(),
        }))
    }

    pub fn new_null_with(databases: Vec<ConfiguredDatabase>) -> Self {
        Self(EnvironmentStrategy::Nulled(EnvironmentStub { databases }))
    }

    pub fn null_builder() -> EnvironmentStubBuilder {
        EnvironmentStubBuilder::default()
    }

    pub fn begin_ro_txn(&self) -> lmdb::Result<RoTransaction> {
        match &self.0 {
            EnvironmentStrategy::Real(s) => s.begin_ro_txn(),
            EnvironmentStrategy::Nulled(s) => s.begin_ro_txn(),
        }
    }

    pub fn begin_rw_txn(&self) -> lmdb::Result<RwTransaction> {
        match &self.0 {
            EnvironmentStrategy::Real(s) => s.begin_rw_txn(),
            EnvironmentStrategy::Nulled(s) => s.begin_rw_txn(),
        }
    }

    pub fn create_db(
        &self,
        name: Option<&str>,
        flags: DatabaseFlags,
    ) -> lmdb::Result<LmdbDatabase> {
        match &self.0 {
            EnvironmentStrategy::Real(s) => s.create_db(name, flags),
            EnvironmentStrategy::Nulled(s) => s.create_db(name, flags),
        }
    }

    pub fn env(&self) -> *mut MDB_env {
        match &self.0 {
            EnvironmentStrategy::Real(s) => s.env(),
            EnvironmentStrategy::Nulled(_) => unimplemented!(),
        }
    }

    pub fn open_db(&self, name: Option<&str>) -> lmdb::Result<LmdbDatabase> {
        match &self.0 {
            EnvironmentStrategy::Real(s) => s.open_db(name),
            EnvironmentStrategy::Nulled(s) => s.open_db(name),
        }
    }

    pub fn sync(&self, force: bool) -> lmdb::Result<()> {
        if let EnvironmentStrategy::Real(s) = &self.0 {
            s.sync(force)?;
        }
        Ok(())
    }

    pub fn stat(&self) -> lmdb::Result<Stat> {
        match &self.0 {
            EnvironmentStrategy::Real(s) => s.stat(),
            EnvironmentStrategy::Nulled(s) => s.stat(),
        }
    }
}

enum EnvironmentStrategy {
    Nulled(EnvironmentStub),
    Real(EnvironmentWrapper),
}

struct EnvironmentWrapper(lmdb::Environment);

impl EnvironmentWrapper {
    fn new(env: lmdb::Environment) -> Self {
        Self(env)
    }

    fn build(options: EnvironmentOptions) -> lmdb::Result<Self> {
        let env = lmdb::Environment::new()
            .set_max_dbs(options.max_dbs)
            .set_map_size(10 * 1024 * 1024)
            .set_flags(options.flags)
            .open_with_permissions(options.path, options.file_mode.try_into().unwrap())?;
        Ok(Self(env))
    }

    fn begin_ro_txn(&self) -> lmdb::Result<RoTransaction> {
        self.0.begin_ro_txn().map(|txn| {
            // todo: don't use static life time
            let txn = unsafe {
                std::mem::transmute::<lmdb::RoTransaction<'_>, lmdb::RoTransaction<'static>>(txn)
            };
            RoTransaction::new(txn)
        })
    }

    fn begin_rw_txn(&self) -> lmdb::Result<RwTransaction> {
        self.0.begin_rw_txn().map(|txn| {
            // todo: don't use static life time
            let txn = unsafe {
                std::mem::transmute::<lmdb::RwTransaction<'_>, lmdb::RwTransaction<'static>>(txn)
            };
            RwTransaction::new(txn)
        })
    }

    fn create_db(&self, name: Option<&str>, flags: DatabaseFlags) -> lmdb::Result<LmdbDatabase> {
        self.0.create_db(name, flags).map(LmdbDatabase::new)
    }

    fn env(&self) -> *mut MDB_env {
        self.0.env()
    }

    fn open_db(&self, name: Option<&str>) -> lmdb::Result<LmdbDatabase> {
        self.0.open_db(name).map(LmdbDatabase::new)
    }

    fn sync(&self, force: bool) -> lmdb::Result<()> {
        self.0.sync(force)
    }

    fn stat(&self) -> lmdb::Result<Stat> {
        self.0.stat()
    }
}

struct EnvironmentStub {
    databases: Vec<ConfiguredDatabase>,
}

impl EnvironmentStub {
    fn begin_ro_txn(&self) -> lmdb::Result<RoTransaction> {
        //todo  don't clone!
        Ok(RoTransaction::new_null(self.databases.clone()))
    }

    fn begin_rw_txn(&self) -> lmdb::Result<RwTransaction> {
        //todo  don't clone!
        Ok(RwTransaction::new_null(self.databases.clone()))
    }

    fn create_db(&self, name: Option<&str>, _flags: DatabaseFlags) -> lmdb::Result<LmdbDatabase> {
        Ok(self
            .databases
            .iter()
            .find(|x| name == Some(&x.db_name))
            .map(|x| x.dbi)
            .unwrap_or(LmdbDatabase::new_null(42)))
    }

    fn open_db(&self, name: Option<&str>) -> lmdb::Result<LmdbDatabase> {
        self.create_db(name, DatabaseFlags::empty())
    }

    fn stat(&self) -> lmdb::Result<Stat> {
        todo!()
    }
}

#[derive(Default)]
pub struct EnvironmentStubBuilder {
    databases: Vec<ConfiguredDatabase>,
}

impl EnvironmentStubBuilder {
    pub fn database(self, name: impl Into<String>, dbi: LmdbDatabase) -> ConfiguredDatabaseBuilder {
        ConfiguredDatabaseBuilder::new(name, dbi, self)
    }

    pub fn configured_database(mut self, db: ConfiguredDatabase) -> Self {
        if self
            .databases
            .iter()
            .any(|x| x.dbi == db.dbi || x.db_name == db.db_name)
        {
            panic!(
                "trying to duplicated database for {} / {}",
                db.dbi.as_nulled(),
                db.db_name
            );
        }
        self.databases.push(db);
        self
    }

    pub fn finish(self) -> LmdbEnvironment {
        LmdbEnvironment::new_null_with(self.databases)
    }
}
