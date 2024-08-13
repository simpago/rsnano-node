use crate::utils::TxnTrackingConfig;

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
}
