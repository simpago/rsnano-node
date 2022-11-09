use std::sync::{Arc, Mutex, Weak};

use indexmap::IndexMap;
use rand::Rng;

use super::{BlockHash, FullHash};

pub struct Uniquer<T>
where
    T: FullHash,
{
    cache: Mutex<IndexMap<BlockHash, Weak<T>>>,
}

impl<T> Uniquer<T>
where
    T: FullHash,
{
    pub(crate) fn new() -> Self {
        Self {
            cache: Mutex::new(IndexMap::new()),
        }
    }

    pub(crate) fn unique(&self, original: &Arc<T>) -> Arc<T> {
        let key = original.full_hash();
        let mut cache = self.cache.lock().unwrap();

        let result = match cache.get(&key) {
            Some(weak) => match weak.upgrade() {
                Some(x) => x,
                None => {
                    cache.insert(key, Arc::downgrade(original));
                    original.clone()
                }
            },
            None => {
                cache.insert(key, Arc::downgrade(original));
                original.clone()
            }
        };

        cleanup(cache);

        result
    }

    pub(crate) fn size(&self) -> usize {
        self.cache.lock().unwrap().len()
    }
}

fn cleanup<T>(mut cache: std::sync::MutexGuard<IndexMap<BlockHash, Weak<T>>>) {
    const CLEANUP_COUNT: usize = 2;
    let mut i = 0;
    while i < CLEANUP_COUNT && !cache.is_empty() {
        let random_offset = rand::thread_rng().gen_range(0..cache.len());
        let mut hash_to_remove = None;
        if let Some((hash, weak)) = cache.get_index(random_offset) {
            if weak.upgrade().is_none() {
                hash_to_remove = Some(*hash);
            }
        }
        if let Some(hash) = &hash_to_remove {
            cache.remove(hash);
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use crate::core::BlockHashBuilder;

    use super::*;

    #[test]
    fn new_item_gets_returned() {
        let uniquer = Uniquer::new();
        let item1 = Arc::new(TestItem(1));
        let item2 = uniquer.unique(&item1);
        assert_eq!(Arc::as_ptr(&item1), Arc::as_ptr(&item2));
    }

    #[test]
    fn when_hashes_are_equal_return_original_item() {
        let uniquer = Uniquer::new();
        let item1 = Arc::new(TestItem(1));
        let item2 = Arc::new(TestItem(1));
        uniquer.unique(&item1);
        let result = uniquer.unique(&item2);
        assert_eq!(Arc::as_ptr(&result), Arc::as_ptr(&item1));
    }

    #[test]
    fn uniquer_holds_weak_references() {
        let uniquer = Uniquer::new();
        let item = Arc::new(TestItem(1));
        let weak = Arc::downgrade(&item);
        drop(uniquer.unique(&item));
        drop(item);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn cleanup() {
        let uniquer = Uniquer::new();
        let item1 = Arc::new(TestItem(1));
        uniquer.unique(&item1);
        {
            let item2 = Arc::new(TestItem(2));
            uniquer.unique(&item2);
        }
        assert_eq!(uniquer.size(), 2);
        let mut iterations = 0;
        while uniquer.size() == 2 {
            uniquer.unique(&item1);
            iterations += 1;
            assert!(iterations < 200);
        }
    }

    struct TestItem(i32);
    impl FullHash for TestItem {
        fn full_hash(&self) -> BlockHash {
            BlockHashBuilder::new().update(self.0.to_ne_bytes()).build()
        }
    }
}
