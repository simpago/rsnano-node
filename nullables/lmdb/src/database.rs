#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub struct LmdbDatabase(DatabaseType);

impl LmdbDatabase {
    pub const fn new(db: lmdb::Database) -> Self {
        Self(DatabaseType::Real(db))
    }

    pub const fn new_null(id: u32) -> Self {
        Self(DatabaseType::Stub(id))
    }

    pub fn as_real(&self) -> lmdb::Database {
        let DatabaseType::Real(db) = &self.0 else {
            panic!("database handle was not a real handle");
        };
        *db
    }

    pub fn as_nulled(&self) -> u32 {
        let DatabaseType::Stub(db) = self.0 else {
            panic!("database handle was not a nulled handle");
        };
        db
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum DatabaseType {
    Real(lmdb::Database),
    Stub(u32),
}
