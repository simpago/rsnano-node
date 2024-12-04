use crate::consensus::ActiveElectionsConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct ActiveElectionsToml {
    pub confirmation_cache: Option<usize>,
    pub confirmation_history_size: Option<usize>,
    pub hinted_limit_percentage: Option<usize>,
    pub optimistic_limit_percentage: Option<usize>,
    pub size: Option<usize>,
}

impl Default for ActiveElectionsToml {
    fn default() -> Self {
        let config = ActiveElectionsConfig::default();
        (&config).into()
    }
}

impl From<&ActiveElectionsToml> for ActiveElectionsConfig {
    fn from(toml: &ActiveElectionsToml) -> Self {
        let mut config = ActiveElectionsConfig::default();

        if let Some(size) = toml.size {
            config.size = size
        };
        if let Some(hinted_limit_percentage) = toml.hinted_limit_percentage {
            config.hinted_limit_percentage = hinted_limit_percentage
        };
        if let Some(optimistic_limit_percentage) = toml.optimistic_limit_percentage {
            config.optimistic_limit_percentage = optimistic_limit_percentage
        };
        if let Some(confirmation_history_size) = toml.confirmation_history_size {
            config.confirmation_history_size = confirmation_history_size
        };
        if let Some(confirmation_cache) = toml.confirmation_cache {
            config.confirmation_cache = confirmation_cache
        };

        config
    }
}

impl From<&ActiveElectionsConfig> for ActiveElectionsToml {
    fn from(config: &ActiveElectionsConfig) -> Self {
        Self {
            size: Some(config.size),
            hinted_limit_percentage: Some(config.hinted_limit_percentage),
            optimistic_limit_percentage: Some(config.optimistic_limit_percentage),
            confirmation_history_size: Some(config.confirmation_history_size),
            confirmation_cache: Some(config.confirmation_cache),
        }
    }
}
