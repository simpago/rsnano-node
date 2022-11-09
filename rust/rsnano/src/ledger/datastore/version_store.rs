use super::{Transaction, WriteTransaction};

pub trait VersionStore {
    fn put(&self, txn: &mut dyn WriteTransaction, version: i32);
    fn get(&self, txn: &dyn Transaction) -> i32;
}
