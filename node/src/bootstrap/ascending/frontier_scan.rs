use std::time::Duration;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FrontierScanConfig {
    pub head_parallelism: usize,
    pub consideration_count: usize,
    pub candidates: usize,
    pub cooldown: Duration,
    pub max_pending: usize,
}

impl Default for FrontierScanConfig {
    fn default() -> Self {
        Self {
            head_parallelism: 128,
            consideration_count: 4,
            candidates: 1000,
            cooldown: Duration::from_secs(5),
            max_pending: 16,
        }
    }
}
