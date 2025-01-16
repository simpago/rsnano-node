use crate::config::NetworkConstants;

#[derive(Clone)]
pub struct NodeConstants {
    pub backup_interval_m: i64,
    pub search_pending_interval_s: i64,
    pub unchecked_cleaning_interval_m: i64,
    pub process_confirmed_interval_ms: i64,
}

impl NodeConstants {
    pub fn new(network_constants: &NetworkConstants) -> Self {
        Self {
            backup_interval_m: 5,
            search_pending_interval_s: if network_constants.is_dev_network() {
                1
            } else {
                5 * 60
            },
            unchecked_cleaning_interval_m: 30,
            process_confirmed_interval_ms: if network_constants.is_dev_network() {
                50
            } else {
                500
            },
        }
    }
}
