use crate::{config::DiagnosticsConfig, utils::TxnTrackingConfig};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct DiagnosticsToml {
    pub txn_tracking: TxnTrackingConfigToml,
}

impl Default for DiagnosticsToml {
    fn default() -> Self {
        Self {
            txn_tracking: TxnTrackingConfigToml::default(),
        }
    }
}

impl From<&DiagnosticsConfig> for DiagnosticsToml {
    fn from(config: &DiagnosticsConfig) -> Self {
        Self {
            txn_tracking: (&config.txn_tracking).into(),
        }
    }
}

impl From<&DiagnosticsToml> for DiagnosticsConfig {
    fn from(config: &DiagnosticsToml) -> Self {
        Self {
            txn_tracking: (&config.txn_tracking).into(),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct TxnTrackingConfigToml {
    pub enable: Option<bool>,
    pub min_read_txn_time_ms: Option<i64>,
    pub min_write_txn_time_ms: Option<i64>,
    pub ignore_writes_below_block_processor_max_time: Option<bool>,
}

impl Default for TxnTrackingConfigToml {
    fn default() -> Self {
        let config = TxnTrackingConfig::default();
        Self {
            enable: Some(config.enable),
            min_read_txn_time_ms: Some(config.min_read_txn_time_ms),
            min_write_txn_time_ms: Some(config.min_write_txn_time_ms),
            ignore_writes_below_block_processor_max_time: Some(
                config.ignore_writes_below_block_processor_max_time,
            ),
        }
    }
}

impl From<&TxnTrackingConfigToml> for TxnTrackingConfig {
    fn from(toml: &TxnTrackingConfigToml) -> Self {
        let mut config = TxnTrackingConfig::default();

        if let Some(enable) = toml.enable {
            config.enable = enable;
        }
        if let Some(ignore_writes_below_block_processor_max_time) =
            toml.ignore_writes_below_block_processor_max_time
        {
            config.ignore_writes_below_block_processor_max_time =
                ignore_writes_below_block_processor_max_time;
        }
        if let Some(min_read_txn_time_ms) = toml.min_read_txn_time_ms {
            config.min_read_txn_time_ms = min_read_txn_time_ms;
        }
        if let Some(min_write_txn_time_ms) = toml.min_write_txn_time_ms {
            config.min_write_txn_time_ms = min_write_txn_time_ms;
        }

        config
    }
}

impl From<&TxnTrackingConfig> for TxnTrackingConfigToml {
    fn from(config: &TxnTrackingConfig) -> Self {
        Self {
            enable: Some(config.enable),
            min_read_txn_time_ms: Some(config.min_read_txn_time_ms),
            min_write_txn_time_ms: Some(config.min_write_txn_time_ms),
            ignore_writes_below_block_processor_max_time: Some(
                config.ignore_writes_below_block_processor_max_time,
            ),
        }
    }
}