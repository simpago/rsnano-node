use crate::utils::TxnTrackingConfig;
use anyhow::Result;
use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
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
