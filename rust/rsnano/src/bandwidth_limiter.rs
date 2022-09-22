use std::sync::Mutex;

use crate::TokenBucket;
pub struct BandwidthLimiter {
    bucket: Mutex<TokenBucket>,
}

impl BandwidthLimiter {
    pub fn new(limit_burst_ratio: f64, limit: usize) -> Self {
        Self {
            bucket: Mutex::new(TokenBucket::new(
                (limit as f64 * limit_burst_ratio) as usize,
                limit,
            )),
        }
    }

    pub fn should_drop(&self, message_size: usize) -> bool {
        !self.bucket.lock().unwrap().try_consume(message_size)
    }

    pub fn reset(&self, limit_burst_ratio: f64, limit: usize) {
        self.bucket
            .lock()
            .unwrap()
            .reset((limit as f64 * limit_burst_ratio) as usize, limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mock_instant::MockClock;
    use std::time::Duration;

    #[test]
    fn test_limit() {
        let limiter = BandwidthLimiter::new(1.5, 10);
        assert_eq!(limiter.should_drop(15), false);
        assert_eq!(limiter.should_drop(1), true);
        MockClock::advance(Duration::from_millis(100));
        assert_eq!(limiter.should_drop(1), false);
        assert_eq!(limiter.should_drop(1), true);
    }
}
