use super::NodeToml;
use crate::{block_processing::BoundedBacklogConfig, config::NodeConfig};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct BoundedBacklogToml {
    pub enable: Option<bool>,
    pub batch_size: Option<usize>,
    pub max_queued_notifications: Option<usize>,
    pub scan_rate: Option<usize>,
}

impl BoundedBacklogConfig {
    pub(crate) fn merge_toml(&mut self, toml: &NodeToml) {
        if let Some(max) = toml.max_backlog {
            self.max_backlog = max;
        }
        let Some(backlog_toml) = &toml.bounded_backlog else {
            return;
        };

        if let Some(size) = backlog_toml.batch_size {
            self.batch_size = size;
        }
        if let Some(max) = backlog_toml.max_queued_notifications {
            self.max_queued_notifications = max;
        }
        if let Some(rate) = backlog_toml.scan_rate {
            self.scan_rate = rate;
        }
    }
}

impl From<&NodeConfig> for BoundedBacklogToml {
    fn from(value: &NodeConfig) -> Self {
        Self {
            enable: Some(value.enable_bounded_backlog),
            batch_size: Some(value.bounded_backlog.batch_size),
            max_queued_notifications: Some(value.bounded_backlog.max_queued_notifications),
            scan_rate: Some(value.bounded_backlog.scan_rate),
        }
    }
}
