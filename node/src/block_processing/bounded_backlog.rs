#[derive(Clone, Debug, PartialEq)]
pub struct BoundedBacklogConfig {
    pub max_backlog: usize,
    pub bucket_threshold: usize,
    pub overfill_factor: f64,
    pub batch_size: usize,
    pub max_queued_notifications: usize,
}

impl Default for BoundedBacklogConfig {
    fn default() -> Self {
        Self {
            max_backlog: 100_000,
            bucket_threshold: 1_000,
            overfill_factor: 1.5,
            batch_size: 32,
            max_queued_notifications: 128,
        }
    }
}
