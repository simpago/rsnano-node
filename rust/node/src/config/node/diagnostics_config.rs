use anyhow::Result;
use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct TxnTrackingConfig {
    /** If true, enable tracking for transaction read/writes held open longer than the min time variables */
    pub enable: bool,
    pub min_read_txn_time_ms: i64,
    pub min_write_txn_time_ms: i64,
    pub ignore_writes_below_block_processor_max_time: bool,
}

impl TxnTrackingConfig {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for TxnTrackingConfig {
    fn default() -> Self {
        Self {
            enable: false,
            min_read_txn_time_ms: 5000,
            min_write_txn_time_ms: 500,
            ignore_writes_below_block_processor_max_time: true,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct TxnTrackingConfigToml {
    pub enable: bool,
    pub min_read_txn_time_ms: i64,
    pub min_write_txn_time_ms: i64,
    pub ignore_writes_below_block_processor_max_time: bool,
}

impl From<TxnTrackingConfig> for TxnTrackingConfigToml {
    fn from(config: TxnTrackingConfig) -> Self {
        Self {
            enable: config.enable,
            min_read_txn_time_ms: config.min_read_txn_time_ms,
            min_write_txn_time_ms: config.min_write_txn_time_ms,
            ignore_writes_below_block_processor_max_time: config
                .ignore_writes_below_block_processor_max_time,
        }
    }
}

#[derive(Clone)]
pub struct DiagnosticsConfig {
    pub txn_tracking: TxnTrackingConfig,
}

impl Default for DiagnosticsConfig {
    fn default() -> Self {
        Self {
            txn_tracking: TxnTrackingConfig::new(),
        }
    }
}

impl DiagnosticsConfig {
    pub fn new() -> Self {
        Default::default()
    }

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

#[derive(Deserialize, Serialize)]
pub struct DiagnosticsConfigToml {
    pub txn_tracking: TxnTrackingConfigToml,
}

impl From<DiagnosticsConfig> for DiagnosticsConfigToml {
    fn from(config: DiagnosticsConfig) -> Self {
        Self {
            txn_tracking: config.txn_tracking.into(),
        }
    }
}
