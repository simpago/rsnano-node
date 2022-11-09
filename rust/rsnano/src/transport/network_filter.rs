use rand::{thread_rng, Rng};
use siphasher::{prelude::*, sip128::SipHasher};
use std::sync::{Mutex, MutexGuard};

/// A probabilistic duplicate filter based on directed map caches, using SipHash 2/4/128
/// The probability of false negatives (unique packet marked as duplicate) is the probability of a 128-bit SipHash collision.
/// The probability of false positives (duplicate packet marked as unique) shrinks with a larger filter.
pub struct NetworkFilter<T: NetworkFilterHasher = DefaultNetworkFilterHasher> {
    items: Mutex<Vec<u128>>,
    hasher: T,
}

impl<T: NetworkFilterHasher> NetworkFilter<T> {
    pub fn with_hasher(hasher: T, size: usize) -> Self {
        Self {
            items: Mutex::new(vec![0; size]),
            hasher,
        }
    }

    /// Reads `count` bytes starting from `bytes` and inserts the siphash digest in the filter.
    /// # Returns
    /// * the resulting siphash digest
    /// * a boolean representing the previous existence of the hash in the filter.
    pub fn apply(&self, bytes: &[u8]) -> (u128, bool) {
        // Get hash before locking
        let digest = self.hash(bytes);

        let mut lock = self.items.lock().unwrap();
        let element = self.get_element(digest, &mut lock);
        let existed = *element == digest;
        if !existed {
            // Replace likely old element with a new one
            *element = digest;
        }

        (digest, existed)
    }

    /// Sets the corresponding element in the filter to zero, if it matches `digest` exactly.
    pub fn clear(&self, digest: u128) {
        let mut lock = self.items.lock().unwrap();
        self.clear_locked(digest, &mut lock);
    }

    pub fn clear_many(&self, digests: impl IntoIterator<Item = u128>) {
        let mut lock = self.items.lock().unwrap();
        for digest in digests.into_iter() {
            self.clear_locked(digest, &mut lock);
        }
    }

    pub fn clear_bytes(&self, bytes: &[u8]) {
        self.clear(self.hash(bytes));
    }

    pub fn clear_all(&self) {
        let mut lock = self.items.lock().unwrap();
        lock.fill(0);
    }

    fn clear_locked(&self, digest: u128, lock: &mut MutexGuard<Vec<u128>>) {
        let element = self.get_element(digest, lock);
        if *element == digest {
            *element = 0;
        }
    }

    fn get_element<'a>(&self, hash: u128, items: &'a mut MutexGuard<Vec<u128>>) -> &'a mut u128 {
        let index = (hash % items.len() as u128) as usize;
        items.get_mut(index).unwrap()
    }

    pub fn hash(&self, bytes: &[u8]) -> u128 {
        self.hasher.hash(bytes)
    }
}

impl NetworkFilter {
    pub fn new(size: usize) -> Self {
        NetworkFilter::with_hasher(DefaultNetworkFilterHasher::new(), size)
    }
}

pub trait NetworkFilterHasher {
    fn hash(&self, bytes: &[u8]) -> u128;
}

pub struct DefaultNetworkFilterHasher {
    key: [u8; 16],
}

impl DefaultNetworkFilterHasher {
    pub fn new() -> Self {
        Self {
            key: thread_rng().gen::<[u8; 16]>(),
        }
    }
}

impl Default for DefaultNetworkFilterHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkFilterHasher for DefaultNetworkFilterHasher {
    fn hash(&self, bytes: &[u8]) -> u128 {
        let mut siphash = SipHasher::new_with_key(&self.key);
        siphash.write(bytes);
        siphash.finish128().as_u128()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct StubHasher {}

    impl NetworkFilterHasher for StubHasher {
        fn hash(&self, bytes: &[u8]) -> u128 {
            bytes[0] as u128
        }
    }

    #[test]
    fn apply_returns_if_key_existed() {
        let filter = NetworkFilter::new(1);
        let bytes = [1, 2, 3];

        let (_, existed) = filter.apply(&bytes);
        assert_eq!(existed, false);

        let (_, existed) = filter.apply(&bytes);
        assert_eq!(existed, true);
    }

    #[test]
    fn clear_bytes() {
        let filter = NetworkFilter::new(1);
        let bytes1 = [1, 2, 3];
        let bytes2 = [1];

        filter.apply(&bytes1);
        filter.clear_bytes(&bytes1);

        let (_, existed) = filter.apply(&bytes1);
        assert_eq!(existed, false);

        let (_, existed) = filter.apply(&bytes1);
        assert_eq!(existed, true);

        filter.clear_bytes(&bytes2);

        let (_, existed) = filter.apply(&bytes1);
        assert_eq!(existed, true);

        let (_, existed) = filter.apply(&bytes2);
        assert_eq!(existed, false);
    }

    #[test]
    fn clear() {
        let filter = NetworkFilter::new(1);
        let bytes = [1, 2, 3];

        let (digest, existed) = filter.apply(&bytes);
        assert_eq!(existed, false);
        assert_ne!(digest, 0);

        let (digest2, existed) = filter.apply(&bytes);
        assert_eq!(existed, true);
        assert_eq!(digest2, digest);

        filter.clear(digest);
        let (_, existed) = filter.apply(&bytes);
        assert_eq!(existed, false);
    }

    #[test]
    fn stub_hasher() {
        assert_eq!(0, StubHasher::default().hash(&[0]));
        assert_eq!(1, StubHasher::default().hash(&[1]));
    }

    #[test]
    fn clear_many() {
        let filter = NetworkFilter::with_hasher(StubHasher::default(), 4);
        let bytes1 = [1];
        let bytes2 = [2];
        let bytes3 = [3];
        let (digest1, _) = filter.apply(&bytes1);
        let (digest2, _) = filter.apply(&bytes2);
        filter.apply(&bytes3);

        filter.clear_many([digest1, digest2]);

        let (_, existed) = filter.apply(&bytes1);
        assert_eq!(existed, false);

        let (_, existed) = filter.apply(&bytes2);
        assert_eq!(existed, false);

        let (_, existed) = filter.apply(&bytes3);
        assert_eq!(existed, true);
    }
}
