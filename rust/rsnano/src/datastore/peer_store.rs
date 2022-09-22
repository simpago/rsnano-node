use crate::{EndpointKey, NoValue};

use super::{DbIterator, Transaction, WriteTransaction};

/// Endpoints for peers
/// nano::endpoint_key -> no_value
pub trait PeerStore {
    fn put(&self, txn: &dyn WriteTransaction, endpoint: &EndpointKey);
    fn del(&self, txn: &dyn WriteTransaction, endpoint: &EndpointKey);
    fn exists(&self, txn: &dyn Transaction, endpoint: &EndpointKey) -> bool;
    fn count(&self, txn: &dyn Transaction) -> usize;
    fn clear(&self, txn: &dyn WriteTransaction);
    fn begin(&self, txn: &dyn Transaction) -> Box<dyn DbIterator<EndpointKey, NoValue>>;
}
