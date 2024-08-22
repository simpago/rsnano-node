use rsnano_network::bandwidth_limiter::OutboundBandwidthLimiterConfig;

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
