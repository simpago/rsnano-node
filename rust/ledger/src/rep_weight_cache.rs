use crate::LedgerCache;
use rsnano_core::{
    utils::{ContainerInfo, ContainerInfoComponent},
    Account, Amount,
};
use std::{
    collections::HashMap,
    mem::size_of,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock, RwLockReadGuard,
    },
};

/// Returns the cached vote weight for the given representative.
/// If the weight is below the cache limit it returns 0.
/// During bootstrap it returns the preconfigured bootstrap weights.
pub struct RepWeightCache {
    weights: Arc<RwLock<HashMap<Account, Amount>>>,
    bootstrap_weights: RwLock<HashMap<Account, Amount>>,
    max_blocks: u64,
    ledger_cache: Arc<LedgerCache>,
    check_bootstrap_weights: AtomicBool,
}

impl RepWeightCache {
    pub fn new() -> Self {
        Self {
            weights: Arc::new(RwLock::new(HashMap::new())),
            bootstrap_weights: RwLock::new(HashMap::new()),
            max_blocks: 0,
            ledger_cache: Arc::new(LedgerCache::new()),
            check_bootstrap_weights: AtomicBool::new(false),
        }
    }

    pub fn with_bootstrap_weights(
        bootstrap_weights: HashMap<Account, Amount>,
        max_blocks: u64,
        ledger_cache: Arc<LedgerCache>,
    ) -> Self {
        Self {
            weights: Arc::new(RwLock::new(HashMap::new())),
            bootstrap_weights: RwLock::new(bootstrap_weights),
            max_blocks,
            ledger_cache,
            check_bootstrap_weights: AtomicBool::new(true),
        }
    }

    pub fn read(&self) -> RwLockReadGuard<HashMap<Account, Amount>> {
        if self.use_bootstrap_weights() {
            self.bootstrap_weights.read().unwrap()
        } else {
            self.weights.read().unwrap()
        }
    }

    pub fn use_bootstrap_weights(&self) -> bool {
        if self.check_bootstrap_weights.load(Ordering::SeqCst) {
            if self.ledger_cache.block_count.load(Ordering::SeqCst) < self.max_blocks {
                return true;
            } else {
                self.check_bootstrap_weights.store(false, Ordering::SeqCst);
            }
        }
        false
    }

    pub fn weight(&self, rep: &Account) -> Amount {
        let weights = if self.use_bootstrap_weights() {
            &self.bootstrap_weights
        } else {
            &self.weights
        };

        weights
            .read()
            .unwrap()
            .get(rep)
            .cloned()
            .unwrap_or_default()
    }

    pub fn bootstrap_weight_max_blocks(&self) -> u64 {
        self.max_blocks
    }

    pub fn bootstrap_weights(&self) -> HashMap<Account, Amount> {
        self.bootstrap_weights.read().unwrap().clone()
    }

    pub fn block_count(&self) -> u64 {
        self.ledger_cache.block_count.load(Ordering::SeqCst)
    }

    pub fn len(&self) -> usize {
        self.weights.read().unwrap().len()
    }

    pub(super) fn inner(&self) -> Arc<RwLock<HashMap<Account, Amount>>> {
        self.weights.clone()
    }

    pub fn collect_container_info(&self, name: impl Into<String>) -> ContainerInfoComponent {
        ContainerInfoComponent::Composite(
            name.into(),
            vec![ContainerInfoComponent::Leaf(ContainerInfo {
                name: "rep_weights".to_string(),
                count: self.len(),
                sizeof_element: size_of::<(Account, Amount)>(),
            })],
        )
    }
}