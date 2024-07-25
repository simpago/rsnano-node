use crate::config::NodeConfig;

use super::{TokenBucket, TrafficType};
use std::sync::Mutex;

impl From<TrafficType> for BandwidthLimitType {
    fn from(value: TrafficType) -> Self {
        match value {
            TrafficType::Generic => BandwidthLimitType::Standard,
            TrafficType::Bootstrap => BandwidthLimitType::Bootstrap,
        }
    }
}

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

    pub fn should_pass(&self, message_size: usize) -> bool {
        self.bucket.lock().unwrap().try_consume(message_size)
    }

    pub fn reset(&self, limit_burst_ratio: f64, limit: usize) {
        self.bucket
            .lock()
            .unwrap()
            .reset((limit as f64 * limit_burst_ratio) as usize, limit)
    }
}

/// Enumeration for different bandwidth limits for different traffic types
#[derive(FromPrimitive)]
pub enum BandwidthLimitType {
    /** For all message */
    Standard,
    /** For bootstrap (asc_pull_ack, asc_pull_req) traffic */
    Bootstrap,
}

pub struct OutboundBandwidthLimiterConfig {
    // standard
    pub standard_limit: usize,
    pub standard_burst_ratio: f64,
    // bootstrap
    pub bootstrap_limit: usize,
    pub bootstrap_burst_ratio: f64,
}

impl Default for OutboundBandwidthLimiterConfig {
    fn default() -> Self {
        Self {
            standard_limit: 10 * 1024 * 1024,
            standard_burst_ratio: 3_f64,
            bootstrap_limit: 5 * 1024 * 1024,
            bootstrap_burst_ratio: 1_f64,
        }
    }
}

impl From<&NodeConfig> for OutboundBandwidthLimiterConfig {
    fn from(value: &NodeConfig) -> Self {
        Self {
            standard_limit: value.bandwidth_limit,
            standard_burst_ratio: value.bandwidth_limit_burst_ratio,
            bootstrap_limit: value.bootstrap_bandwidth_limit,
            bootstrap_burst_ratio: value.bootstrap_bandwidth_burst_ratio,
        }
    }
}

pub struct OutboundBandwidthLimiter {
    limiter_standard: BandwidthLimiter,
    limiter_bootstrap: BandwidthLimiter,
}

impl OutboundBandwidthLimiter {
    pub fn new(config: OutboundBandwidthLimiterConfig) -> Self {
        Self {
            limiter_standard: BandwidthLimiter::new(
                config.standard_burst_ratio,
                config.standard_limit,
            ),
            limiter_bootstrap: BandwidthLimiter::new(
                config.bootstrap_burst_ratio,
                config.bootstrap_limit,
            ),
        }
    }

    /**
     * Check whether packet falls withing bandwidth limits and should be allowed
     * @return true if OK, false if needs to be dropped
     */
    pub fn should_pass(&self, buffer_size: usize, limit_type: BandwidthLimitType) -> bool {
        self.select_limiter(limit_type).should_pass(buffer_size)
    }

    pub fn reset(&self, limit: usize, burst_ratio: f64, limit_type: BandwidthLimitType) {
        self.select_limiter(limit_type).reset(burst_ratio, limit);
    }

    fn select_limiter(&self, limit_type: BandwidthLimitType) -> &BandwidthLimiter {
        match limit_type {
            BandwidthLimitType::Standard => &self.limiter_standard,
            BandwidthLimitType::Bootstrap => &self.limiter_bootstrap,
        }
    }
}

impl Default for OutboundBandwidthLimiter {
    fn default() -> Self {
        Self::new(Default::default())
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
        assert_eq!(limiter.should_pass(15), true);
        assert_eq!(limiter.should_pass(1), false);
        MockClock::advance(Duration::from_millis(100));
        assert_eq!(limiter.should_pass(1), true);
        assert_eq!(limiter.should_pass(1), false);
    }
}
