use crate::BandwidthLimiter;
use std::{ops::Deref, sync::Arc};

pub struct BandwidthLimiterHandle(Arc<BandwidthLimiter>);

impl BandwidthLimiterHandle {
    pub fn new(limiter: Arc<BandwidthLimiter>) -> *mut BandwidthLimiterHandle {
        Box::into_raw(Box::new(BandwidthLimiterHandle(limiter)))
    }
}

impl Deref for BandwidthLimiterHandle {
    type Target = Arc<BandwidthLimiter>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub extern "C" fn rsn_bandwidth_limiter_create(
    limit_burst_ratio: f64,
    limit: usize,
) -> *mut BandwidthLimiterHandle {
    Box::into_raw(Box::new(BandwidthLimiterHandle(Arc::new(
        BandwidthLimiter::new(limit_burst_ratio, limit),
    ))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bandwidth_limiter_destroy(limiter: *mut BandwidthLimiterHandle) {
    drop(Box::from_raw(limiter));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bandwidth_limiter_should_drop(
    limiter: &BandwidthLimiterHandle,
    message_size: usize,
    result: *mut i32,
) -> bool {
    if !result.is_null() {
        *result = 0;
    }

    limiter.0.should_drop(message_size)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bandwidth_limiter_reset(
    limiter: &BandwidthLimiterHandle,
    limit_burst_ratio: f64,
    limit: usize,
) -> i32 {
    limiter.0.reset(limit_burst_ratio, limit);
    0
}
