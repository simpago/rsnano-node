use crate::config::NetworkConstants;
use std::net::Ipv6Addr;

#[derive(Clone)]
pub struct WebsocketConfig {
    pub enabled: bool,
    pub port: u16,
    pub address: String,
}

impl WebsocketConfig {
    pub fn new(network: &NetworkConstants) -> Self {
        Self {
            enabled: false,
            port: network.default_websocket_port,
            address: Ipv6Addr::LOCALHOST.to_string(),
        }
    }
}

impl Default for WebsocketConfig {
    fn default() -> Self {
        Self::new(&NetworkConstants::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsnano_core::work::WorkThresholds;

    #[test]
    fn websocket_config() {
        let cfg = WebsocketConfig::new(&NetworkConstants::new(
            WorkThresholds::publish_full().clone(),
            crate::config::Networks::NanoLiveNetwork,
        ));
        assert_eq!(cfg.enabled, false);
        assert_eq!(cfg.port, 7078);
        assert_eq!(cfg.address, "::1");
    }
}
