use crate::utils::TxnTrackingConfig;
use anyhow::Result;
use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};

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

impl TxnTrackingConfigToml {
    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> Result<()> {
        toml.put_child("txn_tracking", &mut |txn_tracking|{
            txn_tracking.put_bool("enable", self.txn_tracking.enable, "Enable or disable database transaction tracing.\ntype:bool")?;
            txn_tracking.put_i64("min_read_txn_time", self.txn_tracking.min_read_txn_time_ms, "Log stacktrace when read transactions are held longer than this duration.\ntype:milliseconds")?;
            txn_tracking.put_i64("min_write_txn_time", self.txn_tracking.min_write_txn_time_ms, "Log stacktrace when write transactions are held longer than this duration.\ntype:milliseconds")?;
            txn_tracking.put_bool("ignore_writes_below_block_processor_max_time", self.txn_tracking.ignore_writes_below_block_processor_max_time, "Ignore any block processor writes less than block_processor_batch_max_time.\ntype:bool")?;
            Ok(())
        })
    }
}

impl From<&TxnTrackingConfigToml> for TxnTrackingConfig {
    fn from(toml: &TxnTrackingConfigToml) -> Self {
        let mut config = TxnTrackingConfig::default();

        if let Some(enable) = toml.txn_tracking.enable {
            config.enable = enable;
        }
        if let Some(ignore_writes_below_block_processor_max_time) =
            toml.ignore_writes_below_block_processor_max_time
        {
            config.ignore_writes_below_block_processor_max_time =
                ignore_writes_below_block_processor_max_time;
        }
        if let Some(min_read_txn_time_ms) = toml.txn_tracking.min_read_txn_time_ms {
            config.min_read_txn_time_ms = min_read_txn_time_ms;
        }
        if let Some(min_write_txn_time_ms) = toml.txn_tracking.min_write_txn_time_ms {
            config.min_write_txn_time_ms = min_write_txn_time_ms;
        }

        config
    }
}
