use serde::{Deserialize, Serialize};

use crate::{config::TomlConfigOverride, consensus::ActiveElectionsConfig};

#[derive(Deserialize, Serialize)]
pub struct ActiveElectionsConfigToml {
    pub size: Option<usize>,
    pub hinted_limit_percentage: Option<usize>,
    pub optimistic_limit_percentage: Option<usize>,
    pub confirmation_history_size: Option<usize>,
    pub confirmation_cache: Option<usize>,
}

impl From<ActiveElectionsConfig> for ActiveElectionsConfigToml {
    fn from(config: ActiveElectionsConfig) -> Self {
        Self {
            size: Some(config.size),
            hinted_limit_percentage: Some(config.hinted_limit_percentage),
            optimistic_limit_percentage: Some(config.optimistic_limit_percentage),
            confirmation_history_size: Some(config.confirmation_history_size),
            confirmation_cache: Some(config.confirmation_cache),
        }
    }
}

impl<'de> TomlConfigOverride<'de, ActiveElectionsConfigToml> for ActiveElectionsConfig {
    fn toml_config_override(&mut self, toml: &'de ActiveElectionsConfigToml) {
        if let Some(size) = toml.size {
            self.size = size;
        }
        if let Some(hinted_limit_percentage) = toml.hinted_limit_percentage {
            self.hinted_limit_percentage = hinted_limit_percentage;
        }
        if let Some(optimistic_limit_percentage) = toml.optimistic_limit_percentage {
            self.optimistic_limit_percentage = optimistic_limit_percentage;
        }
        if let Some(confirmation_history_size) = toml.confirmation_history_size {
            self.confirmation_history_size = confirmation_history_size;
        }
        if let Some(confirmation_cache) = toml.confirmation_cache {
            self.confirmation_cache = confirmation_cache;
        }
    }
}
