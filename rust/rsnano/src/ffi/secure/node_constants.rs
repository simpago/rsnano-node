use crate::NodeConstants;

#[repr(C)]
pub struct NodeConstantsDto {
    pub backup_interval_m: i64,
    pub search_pending_interval_s: i64,
    pub unchecked_cleaning_interval_m: i64,
    pub process_confirmed_interval_ms: i64,
    pub max_weight_samples: u64,
    pub weight_period: u64,
}

pub fn fill_node_constants_dto(dto: &mut NodeConstantsDto, node: &NodeConstants) {
    dto.backup_interval_m = node.backup_interval_m;
    dto.search_pending_interval_s = node.search_pending_interval_s;
    dto.unchecked_cleaning_interval_m = node.unchecked_cleaning_interval_m;
    dto.process_confirmed_interval_ms = node.process_confirmed_interval_ms;
    dto.max_weight_samples = node.max_weight_samples;
    dto.weight_period = node.weight_period;
}

impl From<&NodeConstantsDto> for NodeConstants {
    fn from(dto: &NodeConstantsDto) -> Self {
        Self {
            backup_interval_m: dto.backup_interval_m,
            search_pending_interval_s: dto.search_pending_interval_s,
            unchecked_cleaning_interval_m: dto.unchecked_cleaning_interval_m,
            process_confirmed_interval_ms: dto.process_confirmed_interval_ms,
            max_weight_samples: dto.max_weight_samples,
            weight_period: dto.weight_period,
        }
    }
}
