use crate::EMPTY_DATABASE;

use super::{ConfiguredDatabase, LmdbDatabase, RoCursor};

pub struct RoTransaction {
    strategy: RoTransactionStrategy,
}

impl RoTransaction {
    pub fn new(tx: lmdb::RoTransaction<'static>) -> Self {
        Self {
            strategy: RoTransactionStrategy::Real(RoTransactionWrapper(tx)),
        }
    }

    pub fn new_null(databases: Vec<ConfiguredDatabase>) -> Self {
        Self {
            strategy: RoTransactionStrategy::Nulled(RoTransactionStub { databases }),
        }
    }

    pub fn reset(self) -> InactiveTransaction {
        match self.strategy {
            RoTransactionStrategy::Real(s) => InactiveTransaction {
                strategy: InactiveTransactionStrategy::Real(s.reset()),
            },
            RoTransactionStrategy::Nulled(s) => InactiveTransaction {
                strategy: InactiveTransactionStrategy::Nulled(s.reset()),
            },
        }
    }

    pub fn commit(self) -> lmdb::Result<()> {
        if let RoTransactionStrategy::Real(s) = self.strategy {
            s.commit()?;
        }
        Ok(())
    }

    pub fn get(&self, database: LmdbDatabase, key: &[u8]) -> lmdb::Result<&[u8]> {
        match &self.strategy {
            RoTransactionStrategy::Real(s) => s.get(database, key),
            RoTransactionStrategy::Nulled(s) => s.get(database, key),
        }
    }

    pub fn open_ro_cursor(&self, database: LmdbDatabase) -> lmdb::Result<RoCursor> {
        match &self.strategy {
            RoTransactionStrategy::Real(s) => s.open_ro_cursor(database),
            RoTransactionStrategy::Nulled(s) => s.open_ro_cursor(database),
        }
    }

    pub fn count(&self, database: LmdbDatabase) -> u64 {
        match &self.strategy {
            RoTransactionStrategy::Real(s) => s.count(database),
            RoTransactionStrategy::Nulled(s) => s.count(database),
        }
    }
}

enum RoTransactionStrategy {
    Real(RoTransactionWrapper),
    Nulled(RoTransactionStub),
}

struct RoTransactionWrapper(lmdb::RoTransaction<'static>);

impl RoTransactionWrapper {
    fn reset(self) -> InactiveTransactionWrapper {
        InactiveTransactionWrapper {
            inactive: self.0.reset(),
        }
    }

    fn commit(self) -> lmdb::Result<()> {
        lmdb::Transaction::commit(self.0)
    }

    fn get(&self, database: LmdbDatabase, key: &[u8]) -> lmdb::Result<&[u8]> {
        lmdb::Transaction::get(&self.0, database.as_real(), &&*key)
    }

    fn open_ro_cursor(&self, database: LmdbDatabase) -> lmdb::Result<RoCursor> {
        lmdb::Transaction::open_ro_cursor(&self.0, database.as_real()).map(|c| {
            //todo don't use static lifetime
            let c =
                unsafe { std::mem::transmute::<lmdb::RoCursor<'_>, lmdb::RoCursor<'static>>(c) };
            RoCursor::new(c)
        })
    }

    fn count(&self, database: LmdbDatabase) -> u64 {
        let stat = lmdb::Transaction::stat(&self.0, database.as_real());
        stat.unwrap().entries() as u64
    }
}

struct RoTransactionStub {
    databases: Vec<ConfiguredDatabase>,
}

impl RoTransactionStub {
    fn get_database(&self, database: LmdbDatabase) -> Option<&ConfiguredDatabase> {
        self.databases.iter().find(|d| d.dbi == database)
    }

    fn reset(self) -> NullInactiveTransaction
    where
        Self: Sized,
    {
        NullInactiveTransaction {
            databases: self.databases,
        }
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

    fn open_ro_cursor<'txn>(&'txn self, database: LmdbDatabase) -> lmdb::Result<RoCursor<'txn>> {
        match self.get_database(database) {
            Some(db) => Ok(RoCursor::new_null(db)),
            None => Ok(RoCursor::new_null(&EMPTY_DATABASE)),
        }
    }

    fn count(&self, database: LmdbDatabase) -> u64 {
        self.get_database(database)
            .map(|db| db.entries.len())
            .unwrap_or_default() as u64
    }
}

pub struct InactiveTransaction {
    strategy: InactiveTransactionStrategy,
}

enum InactiveTransactionStrategy {
    Real(InactiveTransactionWrapper),
    Nulled(NullInactiveTransaction),
}

impl InactiveTransaction {
    pub fn renew(self) -> lmdb::Result<RoTransaction> {
        match self.strategy {
            InactiveTransactionStrategy::Real(s) => Ok(RoTransaction {
                strategy: RoTransactionStrategy::Real(s.renew()?),
            }),
            InactiveTransactionStrategy::Nulled(s) => Ok(RoTransaction {
                strategy: RoTransactionStrategy::Nulled(s.renew()?),
            }),
        }
    }
}

pub struct InactiveTransactionWrapper {
    inactive: lmdb::InactiveTransaction<'static>,
}

impl InactiveTransactionWrapper {
    fn renew(self) -> lmdb::Result<RoTransactionWrapper> {
        self.inactive.renew().map(RoTransactionWrapper)
    }
}

pub struct NullInactiveTransaction {
    databases: Vec<ConfiguredDatabase>,
}

impl NullInactiveTransaction {
    fn renew(self) -> lmdb::Result<RoTransactionStub> {
        Ok(RoTransactionStub {
            databases: self.databases,
        })
    }
}
