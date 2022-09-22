mod account_store;
mod block_store;
mod confirmation_height_store;
mod final_vote_store;
mod frontier_store;
mod ledger;
pub mod lmdb;
mod online_weight_store;
mod peer_store;
mod pending_store;
mod pruned_store;
mod store;
mod unchecked_store;
mod version_store;
mod write_database_queue;

use std::{
    any::Any,
    cmp::{max, min},
};

pub use account_store::AccountStore;
pub use block_store::BlockStore;
pub use confirmation_height_store::ConfirmationHeightStore;
pub use final_vote_store::FinalVoteStore;
pub use frontier_store::FrontierStore;
pub use ledger::Ledger;
pub use online_weight_store::OnlineWeightStore;
pub use peer_store::PeerStore;
pub use pending_store::PendingStore;
use primitive_types::{U256, U512};
pub use pruned_store::PrunedStore;
pub use store::Store;
pub use unchecked_store::UncheckedStore;
pub use version_store::VersionStore;
pub use write_database_queue::{WriteDatabaseQueue, WriteGuard, Writer};

use crate::utils::get_cpu_count;

use self::lmdb::LmdbRawIterator;

pub trait Transaction {
    fn as_any(&self) -> &(dyn Any + '_);
}

pub trait ReadTransaction: Transaction {}

pub trait WriteTransaction: Transaction {
    fn as_transaction(&self) -> &dyn Transaction;
}

pub trait DbIterator<K, V> {
    fn take_lmdb_raw_iterator(&mut self) -> Option<LmdbRawIterator>;
    fn current(&self) -> Option<(&K, &V)>;
    fn value(&self) -> Option<&V>;
    fn next(&mut self);
    fn is_end(&self) -> bool;
}

pub struct NullIterator {}

impl NullIterator {
    pub fn new() -> Self {
        Self {}
    }
}

impl<K, V> DbIterator<K, V> for NullIterator {
    fn take_lmdb_raw_iterator(&mut self) -> Option<LmdbRawIterator> {
        None
    }

    fn is_end(&self) -> bool {
        true
    }

    fn value(&self) -> Option<&V> {
        None
    }

    fn current(&self) -> Option<(&K, &V)> {
        None
    }

    fn next(&mut self) {}
}

pub fn parallel_traversal(action: &(impl Fn(U256, U256, bool) + Send + Sync)) {
    parallel_traversal_impl(U256::max_value(), action);
}

pub fn parallel_traversal_u512(action: &(impl Fn(U512, U512, bool) + Send + Sync)) {
    parallel_traversal_impl(U512::max_value(), action);
}

pub fn parallel_traversal_impl<T>(value_max: T, action: &(impl Fn(T, T, bool) + Send + Sync))
where
    T: std::ops::Div<usize, Output = T> + std::ops::Mul<usize, Output = T> + Send + Copy,
{
    // Between 10 and 40 threads, scales well even in low power systems as long as actions are I/O bound
    let thread_count = max(10, min(40, 11 * get_cpu_count()));
    let split: T = value_max / thread_count;

    std::thread::scope(|s| {
        for thread in 0..thread_count {
            let start = split * thread;
            let end = split * (thread + 1);
            let is_last = thread == thread_count - 1;

            std::thread::Builder::new()
                .name("DB par traversl".to_owned())
                .spawn_scoped(s, move || {
                    action(start, end, is_last);
                })
                .unwrap();
        }
    });
}

pub const STORE_VERSION_MINIMUM: i32 = 21;
pub const STORE_VERSION_CURRENT: i32 = 21;
