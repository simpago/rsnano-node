use super::{ConfiguredDatabase, LmdbDatabase, RoCursor};
use lmdb::DatabaseFlags;

pub struct RwTransaction {
    strategy: RwTransactionStrategy,
}

impl RwTransaction {
    pub fn new(tx: lmdb::RwTransaction<'static>) -> Self {
        Self {
            strategy: RwTransactionStrategy::Real(RwTransactionWrapper(tx)),
        }
    }

    pub fn new_null(databases: Vec<ConfiguredDatabase>) -> Self {
        Self {
            strategy: RwTransactionStrategy::Nulled(RwTransactionStub { databases }),
        }
    }

    pub fn get(&self, database: LmdbDatabase, key: &[u8]) -> lmdb::Result<&[u8]> {
        match &self.strategy {
            RwTransactionStrategy::Real(s) => s.get(database, key),
            RwTransactionStrategy::Nulled(s) => s.get(database, key),
        }
    }

    pub fn put(
        &mut self,
        database: LmdbDatabase,
        key: &[u8],
        data: &[u8],
        flags: lmdb::WriteFlags,
    ) -> lmdb::Result<()> {
        if let RwTransactionStrategy::Real(s) = &mut self.strategy {
            s.put(database.as_real(), key, data, flags)?;
        }
        Ok(())
    }

    pub fn del(
        &mut self,
        database: LmdbDatabase,
        key: &[u8],
        flags: Option<&[u8]>,
    ) -> lmdb::Result<()> {
        if let RwTransactionStrategy::Real(s) = &mut self.strategy {
            s.del(database.as_real(), key, flags)?;
        }
        Ok(())
    }

    /// ## Safety
    ///
    /// This function (as well as `Environment::open_db`,
    /// `Environment::create_db`, and `Database::open`) **must not** be called
    /// from multiple concurrent transactions in the same environment. A
    /// transaction which uses this function must finish (either commit or
    /// abort) before any other transaction may use this function.
    pub unsafe fn create_db(
        &self,
        name: Option<&str>,
        flags: DatabaseFlags,
    ) -> lmdb::Result<LmdbDatabase> {
        match &self.strategy {
            RwTransactionStrategy::Real(s) => s.create_db(name, flags),
            RwTransactionStrategy::Nulled(s) => s.create_db(name, flags),
        }
    }

    /// ## Safety
    ///
    /// This method is unsafe in the same ways as `Environment::close_db`, and
    /// should be used accordingly.
    pub unsafe fn drop_db(&mut self, database: LmdbDatabase) -> lmdb::Result<()> {
        if let RwTransactionStrategy::Real(s) = &mut self.strategy {
            s.drop_db(database.as_real())?;
        }
        Ok(())
    }

    pub fn clear_db(&mut self, database: LmdbDatabase) -> lmdb::Result<()> {
        if let RwTransactionStrategy::Real(s) = &mut self.strategy {
            s.clear_db(database.as_real())?;
        }
        Ok(())
    }

    pub fn open_ro_cursor(&self, database: LmdbDatabase) -> lmdb::Result<RoCursor> {
        match &self.strategy {
            RwTransactionStrategy::Real(s) => s.open_ro_cursor(database),
            RwTransactionStrategy::Nulled(s) => s.open_ro_cursor(database),
        }
    }

    pub fn count(&self, database: LmdbDatabase) -> u64 {
        match &self.strategy {
            RwTransactionStrategy::Real(s) => s.count(database.as_real()),
            RwTransactionStrategy::Nulled(_) => 0,
        }
    }

    pub fn commit(self) -> lmdb::Result<()> {
        if let RwTransactionStrategy::Real(s) = self.strategy {
            s.commit()?;
        }
        Ok(())
    }
}

enum RwTransactionStrategy {
    Real(RwTransactionWrapper),
    Nulled(RwTransactionStub),
}

pub struct RwTransactionWrapper(lmdb::RwTransaction<'static>);

impl RwTransactionWrapper {
    fn get(&self, database: LmdbDatabase, key: &[u8]) -> lmdb::Result<&[u8]> {
        lmdb::Transaction::get(&self.0, database.as_real(), &key)
    }

    fn put(
        &mut self,
        database: lmdb::Database,
        key: &[u8],
        data: &[u8],
        flags: lmdb::WriteFlags,
    ) -> lmdb::Result<()> {
        lmdb::RwTransaction::put(&mut self.0, database, &key, &data, flags)
    }

    fn del(
        &mut self,
        database: lmdb::Database,
        key: &[u8],
        flags: Option<&[u8]>,
    ) -> lmdb::Result<()> {
        lmdb::RwTransaction::del(&mut self.0, database, &key, flags)
    }

    fn clear_db(&mut self, database: lmdb::Database) -> lmdb::Result<()> {
        lmdb::RwTransaction::clear_db(&mut self.0, database)
    }

    fn commit(self) -> lmdb::Result<()> {
        lmdb::Transaction::commit(self.0)
    }

    fn open_ro_cursor(&self, database: LmdbDatabase) -> lmdb::Result<RoCursor> {
        let cursor = lmdb::Transaction::open_ro_cursor(&self.0, database.as_real());
        cursor.map(RoCursor::new)
    }

    fn count(&self, database: lmdb::Database) -> u64 {
        let stat = lmdb::Transaction::stat(&self.0, database);
        stat.unwrap().entries() as u64
    }

    /// ## Safety
    ///
    /// This method is unsafe in the same ways as `Environment::close_db`, and
    /// should be used accordingly.
    unsafe fn drop_db(&mut self, database: lmdb::Database) -> lmdb::Result<()> {
        lmdb::RwTransaction::drop_db(&mut self.0, database)
    }

    /// ## Safety
    ///
    /// This function (as well as `Environment::open_db`,
    /// `Environment::create_db`, and `Database::open`) **must not** be called
    /// from multiple concurrent transactions in the same environment. A
    /// transaction which uses this function must finish (either commit or
    /// abort) before any other transaction may use this function.
    unsafe fn create_db(
        &self,
        name: Option<&str>,
        flags: DatabaseFlags,
    ) -> lmdb::Result<LmdbDatabase> {
        lmdb::RwTransaction::create_db(&self.0, name, flags).map(LmdbDatabase::new)
    }
}

pub struct RwTransactionStub {
    databases: Vec<ConfiguredDatabase>,
}

impl RwTransactionStub {
    fn get_database(&self, database: LmdbDatabase) -> Option<&ConfiguredDatabase> {
        self.databases.iter().find(|d| d.dbi == database)
    }

    fn get(&self, database: LmdbDatabase, key: &[u8]) -> lmdb::Result<&[u8]> {
        let Some(db) = self.get_database(database) else {
            return Err(lmdb::Error::NotFound);
        };
        match db.entries.get(key) {
            Some(value) => Ok(value),
            None => Err(lmdb::Error::NotFound),
        }
    }

    fn open_ro_cursor(&self, database: LmdbDatabase) -> lmdb::Result<RoCursor> {
        Ok(RoCursor::new_null(
            self.databases.iter().find(|db| db.dbi == database).unwrap(),
        ))
    }

    fn create_db(&self, _name: Option<&str>, _flags: DatabaseFlags) -> lmdb::Result<LmdbDatabase> {
        Ok(LmdbDatabase::new_null(42))
    }
}
