use super::Miliseconds;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct MonitorConfigToml {
    pub enabled: bool,
    pub interval: Miliseconds,
}
