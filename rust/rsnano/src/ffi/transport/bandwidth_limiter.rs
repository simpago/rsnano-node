use crate::transport::{
    BandwidthLimitType, BandwidthLimiter, OutboundBandwidthLimiter, OutboundBandwidthLimiterConfig,
};
use num_traits::FromPrimitive;
use std::{borrow::Borrow, ops::Deref, sync::Arc};

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
pub unsafe extern "C" fn rsn_bandwidth_limiter_should_pass(
    limiter: &BandwidthLimiterHandle,
    message_size: usize,
) -> bool {
    limiter.0.should_pass(message_size)
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

pub struct OutboundBandwidthLimiterHandle(Arc<OutboundBandwidthLimiter>);

impl Deref for OutboundBandwidthLimiterHandle {
    type Target = Arc<OutboundBandwidthLimiter>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[repr(C)]
pub struct OutboundBandwidthLimiterConfigDto {
    pub standard_limit: usize,
    pub standard_burst_ratio: f64,
    pub bootstrap_limit: usize,
    pub bootstrap_burst_ratio: f64,
}

impl From<&OutboundBandwidthLimiterConfigDto> for OutboundBandwidthLimiterConfig {
    fn from(dto: &OutboundBandwidthLimiterConfigDto) -> Self {
        Self {
            standard_limit: dto.standard_limit,
            standard_burst_ratio: dto.standard_burst_ratio,
            bootstrap_limit: dto.bootstrap_limit,
            bootstrap_burst_ratio: dto.bootstrap_burst_ratio,
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_outbound_bandwidth_limiter_create(
    config: *const OutboundBandwidthLimiterConfigDto,
) -> *mut OutboundBandwidthLimiterHandle {
    let config = (*config).borrow().into();
    Box::into_raw(Box::new(OutboundBandwidthLimiterHandle(Arc::new(
        OutboundBandwidthLimiter::new(config),
    ))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_outbound_bandwidth_limiter_destroy(
    limiter: *mut OutboundBandwidthLimiterHandle,
) {
    drop(Box::from_raw(limiter));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_outbound_bandwidth_limiter_should_pass(
    limiter: &OutboundBandwidthLimiterHandle,
    message_size: usize,
    limit_type: u8,
) -> bool {
    limiter.0.should_pass(
        message_size,
        BandwidthLimitType::from_u8(limit_type).unwrap(),
    )
}

#[no_mangle]
pub unsafe extern "C" fn rsn_outbound_bandwidth_limiter_reset(
    limiter: &OutboundBandwidthLimiterHandle,
    limit_burst_ratio: f64,
    limit: usize,
    limit_type: u8,
) {
    limiter.0.reset(
        limit,
        limit_burst_ratio,
        BandwidthLimitType::from_u8(limit_type).unwrap(),
    );
}
