use crate::BootstrapConstants;

#[repr(C)]
pub struct BootstrapConstantsDto {
    pub lazy_max_pull_blocks: u32,
    pub lazy_min_pull_blocks: u32,
    pub frontier_retry_limit: u32,
    pub lazy_retry_limit: u32,
    pub lazy_destinations_retry_limit: u32,
    pub gap_cache_bootstrap_start_interval_ms: i64,
    pub default_frontiers_age_seconds: u32,
}

pub fn fill_bootstrap_constants_dto(
    dto: &mut BootstrapConstantsDto,
    bootstrap: &BootstrapConstants,
) {
    dto.lazy_max_pull_blocks = bootstrap.lazy_max_pull_blocks;
    dto.lazy_min_pull_blocks = bootstrap.lazy_min_pull_blocks;
    dto.frontier_retry_limit = bootstrap.frontier_retry_limit;
    dto.lazy_retry_limit = bootstrap.lazy_retry_limit;
    dto.lazy_destinations_retry_limit = bootstrap.lazy_destinations_retry_limit;
    dto.gap_cache_bootstrap_start_interval_ms = bootstrap.gap_cache_bootstrap_start_interval_ms;
    dto.default_frontiers_age_seconds = bootstrap.default_frontiers_age_seconds;
}

impl From<&BootstrapConstantsDto> for BootstrapConstants {
    fn from(dto: &BootstrapConstantsDto) -> Self {
        Self {
            lazy_max_pull_blocks: dto.lazy_max_pull_blocks,
            lazy_min_pull_blocks: dto.lazy_min_pull_blocks,
            frontier_retry_limit: dto.frontier_retry_limit,
            lazy_retry_limit: dto.lazy_retry_limit,
            lazy_destinations_retry_limit: dto.lazy_destinations_retry_limit,
            gap_cache_bootstrap_start_interval_ms: dto.gap_cache_bootstrap_start_interval_ms,
            default_frontiers_age_seconds: dto.default_frontiers_age_seconds,
        }
    }
}
