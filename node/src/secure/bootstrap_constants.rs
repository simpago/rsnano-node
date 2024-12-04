use crate::config::NetworkConstants;

#[derive(Clone)]
pub struct BootstrapConstants {
    pub lazy_max_pull_blocks: u32,
    pub lazy_min_pull_blocks: u32,
    pub frontier_retry_limit: u32,
    pub lazy_retry_limit: u32,
    pub lazy_destinations_retry_limit: u32,
    pub gap_cache_bootstrap_start_interval_ms: i64,
    pub default_frontiers_age_seconds: u32,
}

impl BootstrapConstants {
    pub fn new(network_constants: &NetworkConstants) -> Self {
        let frontier_retry_limit = if network_constants.is_dev_network() {
            2
        } else {
            16
        };
        Self {
            lazy_max_pull_blocks: if network_constants.is_dev_network() {
                2
            } else {
                512
            },
            lazy_min_pull_blocks: if network_constants.is_dev_network() {
                1
            } else {
                32
            },
            frontier_retry_limit,
            lazy_retry_limit: if network_constants.is_dev_network() {
                2
            } else {
                frontier_retry_limit * 4
            },
            lazy_destinations_retry_limit: if network_constants.is_dev_network() {
                1
            } else {
                frontier_retry_limit / 4
            },
            gap_cache_bootstrap_start_interval_ms: if network_constants.is_dev_network() {
                5
            } else {
                30 * 1000
            },
            default_frontiers_age_seconds: if network_constants.is_dev_network() {
                1
            } else {
                24 * 60 * 60
            }, // 1 second for dev network, 24 hours for live/beta
        }
    }
}
