use crate::config::MonitorConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct MonitorToml {
    pub enable: Option<bool>,
    pub interval: Option<u64>,
}

impl Default for MonitorToml {
    fn default() -> Self {
        let config = MonitorConfig::default();
        Self {
            enable: Some(true),
            interval: Some(config.interval.as_secs()),
        }
    }
}
