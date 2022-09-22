use std::{collections::HashMap, sync::Arc};

use super::BootstrapStrategy;

/// WARNING: BootstrapAttempts is not connected to the C++ version yet!
/// Container for bootstrap sessions that are active. Owned by `BootstrapInitiator`.
pub struct BootstrapAttempts {
    incremental: usize,
    attempts: HashMap<usize, Arc<BootstrapStrategy>>,
}

impl BootstrapAttempts {
    pub fn new() -> Self {
        Self {
            incremental: 0,
            attempts: HashMap::new(),
        }
    }

    pub fn get_incremental_id(&mut self) -> usize {
        let id = self.incremental;
        self.incremental += 1;
        id
    }

    pub fn add(&mut self, attempt: Arc<BootstrapStrategy>) {
        let id = self.get_incremental_id();
        self.attempts.insert(id, attempt);
    }

    pub fn remove(&mut self, incremental_id: usize) {
        self.attempts.remove(&incremental_id);
    }

    pub fn clear(&mut self) {
        self.attempts.clear();
    }

    pub fn find(&self, incremental_id: usize) -> Option<&Arc<BootstrapStrategy>> {
        self.attempts.get(&incremental_id)
    }

    pub fn size(&self) -> usize {
        self.attempts.len()
    }

    pub fn total_attempts(&self) -> usize {
        self.incremental
    }

    pub fn attempts(&self) -> &HashMap<usize, Arc<BootstrapStrategy>> {
        &self.attempts
    }
}
