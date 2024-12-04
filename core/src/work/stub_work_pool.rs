use super::WorkPool;
use crate::Root;

/// The StubWorkPool assumes work == difficulty
pub struct StubWorkPool {
    base_difficulty: u64,
}

impl StubWorkPool {
    pub fn new(base_difficulty: u64) -> Self {
        Self { base_difficulty }
    }
}

impl Default for StubWorkPool {
    fn default() -> Self {
        Self::new(123)
    }
}

impl WorkPool for StubWorkPool {
    fn generate_async(
        &self,
        _root: Root,
        difficulty: u64,
        done: Option<Box<dyn FnOnce(Option<u64>) + Send>>,
    ) {
        if let Some(done) = done {
            done(Some(difficulty))
        }
    }

    fn generate_dev(&self, _root: Root, difficulty: u64) -> Option<u64> {
        Some(difficulty)
    }

    fn generate_dev2(&self, _root: Root) -> Option<u64> {
        Some(self.base_difficulty)
    }

    fn generate(&self, _root: Root, difficulty: u64) -> Option<u64> {
        Some(difficulty)
    }
}
