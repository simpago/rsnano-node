use std::{
    collections::{HashMap, VecDeque},
    sync::Mutex,
};

use rsnano_core::{
    utils::{ContainerInfo, ContainerInfoComponent, ContainerInfos},
    BlockHash, QualifiedRoot,
};

pub struct RecentlyConfirmedCache {
    mutex: Mutex<RecentlyConfirmedCacheImpl>,
    max_len: usize,
}

impl RecentlyConfirmedCache {
    pub fn new(max_len: usize) -> Self {
        Self {
            mutex: Mutex::new(RecentlyConfirmedCacheImpl {
                sequential: VecDeque::new(),
                by_root: HashMap::new(),
                by_hash: HashMap::new(),
            }),
            max_len,
        }
    }

    pub fn put(&self, root: QualifiedRoot, hash: BlockHash) -> bool {
        let mut guard = self.mutex.lock().unwrap();
        if guard.by_hash.contains_key(&hash) || guard.by_root.contains_key(&root) {
            return false;
        }
        guard.sequential.push_back(hash);
        guard.by_root.insert(root.clone(), hash);
        guard.by_hash.insert(hash, root);
        if guard.sequential.len() > self.max_len {
            if let Some(old_hash) = guard.sequential.pop_front() {
                if let Some(old_root) = guard.by_hash.remove(&old_hash) {
                    guard.by_root.remove(&old_root);
                }
            }
        }
        true
    }

    pub fn erase(&self, hash: &BlockHash) {
        let mut guard = self.mutex.lock().unwrap();
        if let Some(root) = guard.by_hash.remove(hash) {
            guard.by_root.remove(&root);
            guard.sequential.retain(|i| i != hash);
        }
    }

    pub fn root_exists(&self, root: &QualifiedRoot) -> bool {
        self.mutex.lock().unwrap().by_root.contains_key(root)
    }

    pub fn hash_exists(&self, hash: &BlockHash) -> bool {
        self.mutex.lock().unwrap().by_hash.contains_key(hash)
    }

    pub fn clear(&self) {
        let mut guard = self.mutex.lock().unwrap();
        guard.sequential.clear();
        guard.by_root.clear();
        guard.by_hash.clear();
    }

    pub fn len(&self) -> usize {
        self.mutex.lock().unwrap().sequential.len()
    }

    pub fn back(&self) -> Option<(QualifiedRoot, BlockHash)> {
        let guard = self.mutex.lock().unwrap();
        guard
            .sequential
            .back()
            .map(|hash| (guard.by_hash.get(hash).unwrap().clone(), *hash))
    }

    pub fn container_info(&self) -> ContainerInfos {
        [(
            "confirmed",
            self.len(),
            std::mem::size_of::<BlockHash>() * 3 + std::mem::size_of::<QualifiedRoot>(),
        )]
        .into()
    }
}

struct RecentlyConfirmedCacheImpl {
    by_root: HashMap<QualifiedRoot, BlockHash>,
    by_hash: HashMap<BlockHash, QualifiedRoot>,
    sequential: VecDeque<BlockHash>,
}

impl RecentlyConfirmedCacheImpl {}
