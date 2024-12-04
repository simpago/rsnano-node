use crate::config::NetworkConstants;

#[derive(Clone)]
pub struct PortmappingConstants {
    pub lease_duration_s: i64,
    pub health_check_period_s: i64,
}

impl PortmappingConstants {
    pub fn new(_: &NetworkConstants) -> Self {
        Self {
            lease_duration_s: 1787, // ~30 minutes
            health_check_period_s: 53,
        }
    }
}
