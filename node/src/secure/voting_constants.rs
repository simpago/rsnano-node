use crate::config::NetworkConstants;

#[derive(Clone)]
pub struct VotingConstants {
    pub max_cache: usize,
    pub delay_s: i64,
}

impl VotingConstants {
    pub fn new(network_constants: &NetworkConstants) -> Self {
        Self {
            max_cache: if network_constants.is_dev_network() {
                256
            } else {
                128 * 1024
            },
            delay_s: if network_constants.is_dev_network() {
                1
            } else {
                15
            },
        }
    }
}
