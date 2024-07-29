use crate::consensus::ActiveElectionsConfig;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct ActiveElectionsConfigToml {
    pub size: Option<usize>,
    pub hinted_limit_percentage: Option<usize>,
    pub optimistic_limit_percentage: Option<usize>,
    pub confirmation_history_size: Option<usize>,
    pub confirmation_cache: Option<usize>,
}

impl Default for ActiveElectionsConfigToml {
    fn default() -> Self {
        let config = ActiveElectionsConfig::default();
        Self {
            size: Some(config.size),
            hinted_limit_percentage: Some(config.hinted_limit_percentage),
            optimistic_limit_percentage: Some(config.optimistic_limit_percentage),
            confirmation_history_size: Some(config.confirmation_history_size),
            confirmation_cache: Some(config.confirmation_cache),
        }
    }
}

impl From<&ActiveElectionsConfigToml> for ActiveElectionsConfig {
    fn from(toml: &ActiveElectionsConfigToml) -> Self {
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

impl From<&ActiveElectionsConfig> for ActiveElectionsConfigToml {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nullable_fs::NullableFilesystem;
    use std::path::PathBuf;

    #[test]
    fn config_to_toml() {
        let mut config = ActiveElectionsConfig::default();

        config.confirmation_cache = 0;

        let toml_config: ActiveElectionsConfigToml = (&config).into();

        assert_eq!(toml_config.size, Some(config.size));
        assert_eq!(
            toml_config.hinted_limit_percentage,
            Some(config.hinted_limit_percentage)
        );
        assert_eq!(
            toml_config.optimistic_limit_percentage,
            Some(config.optimistic_limit_percentage)
        );
        assert_eq!(
            toml_config.confirmation_history_size,
            Some(config.confirmation_history_size)
        );
        assert_eq!(toml_config.confirmation_cache, Some(0));
    }

    #[test]
    fn toml_to_config() {
        let toml_write = r#"
                size = 30
                hinted_limit_percentage = 70
                optimistic_limit_percentage = 85
                confirmation_history_size = 300
                confirmation_cache = 3000
            "#;

        let path: PathBuf = "/tmp/config-node.toml".into();

        NullableFilesystem::new().write(&path, toml_write).unwrap();

        let toml_read = NullableFilesystem::new().read_to_string(&path).unwrap();

        let toml_config: ActiveElectionsConfigToml =
            toml::from_str(&toml_read).expect("Failed to deserialize TOML");

        let config: ActiveElectionsConfig = (&toml_config).into();

        assert_eq!(config.size, 30);
        assert_eq!(config.hinted_limit_percentage, 70);
        assert_eq!(config.optimistic_limit_percentage, 85);
        assert_eq!(config.confirmation_history_size, 300);
        assert_eq!(config.confirmation_cache, 3000);
    }

    #[test]
    fn toml_with_comments_to_config() {
        let toml_write = r#"
                size = 40
                optimistic_limit_percentage = 90
                # confirmation_cache = 4000
            "#;

        let path: PathBuf = "/tmp/config-node.toml".into();

        NullableFilesystem::new().write(&path, toml_write).unwrap();

        let toml_read = NullableFilesystem::new().read_to_string(&path).unwrap();

        let toml_config: ActiveElectionsConfigToml =
            toml::from_str(&toml_read).expect("Failed to deserialize TOML");

        let config: ActiveElectionsConfig = (&toml_config).into();

        assert_eq!(config.size, 40);
        assert_eq!(
            config.hinted_limit_percentage,
            config.hinted_limit_percentage
        );
        assert_eq!(config.optimistic_limit_percentage, 90);
        assert_eq!(
            config.confirmation_history_size,
            config.confirmation_history_size
        );
        assert_eq!(config.confirmation_cache, config.confirmation_cache);
    }

    #[test]
    fn toml_empty_to_config() {
        let toml_write = r#""#;

        let path: PathBuf = "/tmp/config-node.toml".into();

        NullableFilesystem::new().write(&path, toml_write).unwrap();

        let toml_read = NullableFilesystem::new().read_to_string(&path).unwrap();

        let toml_config: ActiveElectionsConfigToml =
            toml::from_str(&toml_read).expect("Failed to deserialize TOML");

        let config: ActiveElectionsConfig = (&toml_config).into();

        assert_eq!(config.size, config.size);
        assert_eq!(
            config.hinted_limit_percentage,
            config.hinted_limit_percentage
        );
        assert_eq!(
            config.optimistic_limit_percentage,
            config.optimistic_limit_percentage
        );
        assert_eq!(
            config.confirmation_history_size,
            config.confirmation_history_size
        );
        assert_eq!(config.confirmation_cache, config.confirmation_cache);
    }
}
