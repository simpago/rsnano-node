use crate::block_processing::BacklogPopulationConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct BacklogPopulationToml {
    pub enable: Option<bool>,
    pub batch_size: Option<u32>,
    pub frequency: Option<u32>,
}

impl From<&BacklogPopulationConfig> for BacklogPopulationToml {
    fn from(value: &BacklogPopulationConfig) -> Self {
        Self {
            enable: Some(value.enabled),
            batch_size: Some(value.batch_size),
            frequency: Some(value.frequency),
        }
    }
}

impl BacklogPopulationConfig {
    pub(crate) fn merge_toml(&mut self, toml: &BacklogPopulationToml) {
        if let Some(enable) = toml.enable {
            self.enabled = enable;
        }

        if let Some(size) = toml.batch_size {
            self.batch_size = size;
        }

        if let Some(freq) = toml.frequency {
            self.frequency = freq;
        }
    }
}
