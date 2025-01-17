use std::time::Duration;

use crate::config::TcpConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct TcpToml {
    pub max_inbound_connections: Option<usize>,
    pub max_outbound_connections: Option<usize>,
    pub max_attempts: Option<usize>,
    pub max_attempts_per_ip: Option<usize>,
    pub connect_timeout: Option<usize>,
    pub handshake_timeout: Option<usize>,
    pub io_timeout: Option<usize>,
}

impl From<&TcpConfig> for TcpToml {
    fn from(value: &TcpConfig) -> Self {
        Self {
            max_inbound_connections: Some(value.max_inbound_connections),
            max_outbound_connections: Some(value.max_outbound_connections),
            max_attempts: Some(value.max_attempts),
            max_attempts_per_ip: Some(value.max_attempts_per_ip),
            connect_timeout: None,
            handshake_timeout: None,
            io_timeout: None,
        }
    }
}

impl TcpConfig {
    pub fn merge_toml(&mut self, toml: &TcpToml) {
        if let Some(i) = toml.max_inbound_connections {
            self.max_inbound_connections = i;
        }
        if let Some(i) = toml.max_outbound_connections {
            self.max_outbound_connections = i;
        }
        if let Some(i) = toml.max_attempts {
            self.max_attempts = i;
        }
        if let Some(i) = toml.max_attempts_per_ip {
            self.max_attempts_per_ip = i;
        }
        if let Some(i) = toml.connect_timeout {
            self.connect_timeout = Duration::from_secs(i as u64)
        }
    }
}
