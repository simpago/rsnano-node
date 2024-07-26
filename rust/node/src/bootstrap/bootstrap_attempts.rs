use rsnano_core::utils::PropertyTree;

use crate::utils::create_property_tree;

use super::BootstrapStrategy;
use std::{
    collections::HashMap,
    sync::{atomic::Ordering, Arc},
    usize,
};

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
        self.attempts
            .insert(attempt.incremental_id() as usize, attempt);
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

    pub fn attempts_information(&self, attempts: &mut dyn PropertyTree) {
        for (_, attempt) in &self.attempts {
            let mut entry = create_property_tree();
            entry.put_string("id", attempt.id()).unwrap();
            entry.put_string("mode", attempt.mode().as_str()).unwrap();
            entry
                .put_string("started", if attempt.started() { "true" } else { "false" })
                .unwrap();
            entry
                .put_string(
                    "pulling",
                    &attempt.attempt().pulling.load(Ordering::SeqCst).to_string(),
                )
                .unwrap();
            entry
                .put_string(
                    "total_blocks",
                    &attempt
                        .attempt()
                        .total_blocks
                        .load(Ordering::SeqCst)
                        .to_string(),
                )
                .unwrap();
            entry
                .put_string(
                    "requeued_pulls",
                    &attempt
                        .attempt()
                        .requeued_pulls
                        .load(Ordering::SeqCst)
                        .to_string(),
                )
                .unwrap();
            attempt.get_information(&mut *entry);
            entry
                .put_u64("duration", attempt.attempt().duration().as_secs() as u64)
                .unwrap();
            attempts.push_back("", &*entry);
        }
    }
}
