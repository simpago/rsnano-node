use anyhow::Result;
use once_cell::sync::Lazy;
use rsnano_core::{
    utils::get_env_or_default,
    work::{WorkThresholds, WORK_THRESHOLDS_STUB},
    Networks,
};
use std::{sync::Mutex, time::Duration};

use crate::messages::ProtocolInfo;

//todo: make configurable in builld script again!
static ACTIVE_NETWORK: Lazy<Mutex<Networks>> = Lazy::new(|| Mutex::new(Networks::NanoDevNetwork));

pub static STUB_NETWORK_CONSTANTS: Lazy<NetworkConstants> =
    Lazy::new(|| NetworkConstants::new(WORK_THRESHOLDS_STUB.clone(), Networks::NanoDevNetwork));

#[derive(Clone)]
pub struct NetworkConstants {
    pub work: WorkThresholds,
    pub default_node_port: u16,
    pub default_rpc_port: u16,
    pub default_ipc_port: u16,
    pub default_websocket_port: u16,
    pub aec_loop_interval_ms: u32,
    pub cleanup_period_s: i64,
    /** How often to send keepalive messages */
    pub keepalive_period: Duration,
    /** Default maximum idle time for a socket before it's automatically closed */
    pub idle_timeout_s: i64,
    pub sync_cookie_cutoff_s: i64,
    pub bootstrap_interval_s: i64,
    /** Maximum number of peers per IP. It is also the max number of connections per IP*/
    pub max_peers_per_ip: usize,
    /** Maximum number of peers per subnetwork */
    pub max_peers_per_subnetwork: usize,
    pub peer_dump_interval_s: i64,

    pub current_network: Networks,
    /** Current protocol version */
    pub protocol_version: u8,
    /** Minimum accepted protocol version */
    pub protocol_version_min: u8,
    /** Minimum accepted protocol version used when bootstrapping */
    pub bootstrap_protocol_version_min: u8,
    pub ipv6_subnetwork_prefix_for_limiting: usize,
    pub silent_connection_tolerance_time_s: i64,
    /// Time to wait before vote rebroadcasts for active elections (milliseconds)
    pub vote_broadcast_interval_ms: i64,

    /** We do not reply to telemetry requests made within cooldown period */
    pub telemetry_request_cooldown_ms: i64,
    /** How often to request telemetry from peers */
    pub telemetry_request_interval_ms: i64,
    /** How often to broadcast telemetry to peers */
    pub telemetry_broadcast_interval_ms: i64,
    /** Telemetry data older than this value is considered stale */
    pub telemetry_cache_cutoff_ms: i64, // 2 * `telemetry_broadcast_interval` + some margin
    /// How much to delay activation of optimistic elections to avoid interfering with election scheduler
    pub optimistic_activation_delay: Duration,
}

impl NetworkConstants {
    pub fn empty() -> Self {
        Self::new(WorkThresholds::publish_dev().clone(), Networks::Invalid)
    }

    pub fn new(work: WorkThresholds, network: Networks) -> Self {
        match network {
            Networks::NanoDevNetwork => Self::dev(work),
            Networks::NanoBetaNetwork => Self::beta(work),
            Networks::NanoLiveNetwork | Networks::Invalid => Self::live(work),
            Networks::NanoTestNetwork => Self::test(work),
        }
    }

    pub fn protocol_info(&self) -> ProtocolInfo {
        ProtocolInfo {
            version_using: self.protocol_version,
            version_max: self.protocol_version,
            version_min: self.protocol_version_min,
            network: self.current_network,
        }
    }

    fn live(work: WorkThresholds) -> Self {
        let cleanup_period_s = 60;
        let max_peers_per_ip = 10;
        let protocol_info = ProtocolInfo::default();
        Self {
            work,
            current_network: Networks::NanoLiveNetwork,
            protocol_version: protocol_info.version_using,
            protocol_version_min: protocol_info.version_min,
            bootstrap_protocol_version_min: 0x13,
            default_node_port: 7075,
            default_rpc_port: 7076,
            default_ipc_port: 7077,
            default_websocket_port: 7078,
            aec_loop_interval_ms: 300,
            cleanup_period_s,
            keepalive_period: Duration::from_secs(15),
            idle_timeout_s: cleanup_period_s * 2,
            sync_cookie_cutoff_s: 5,
            bootstrap_interval_s: 15 * 60,
            max_peers_per_ip,
            max_peers_per_subnetwork: max_peers_per_ip * 4,
            peer_dump_interval_s: 5 * 60,
            ipv6_subnetwork_prefix_for_limiting: 64,
            silent_connection_tolerance_time_s: 120,
            vote_broadcast_interval_ms: 15 * 1000,
            telemetry_request_cooldown_ms: 1000 * 15,
            telemetry_request_interval_ms: 1000 * 60,
            telemetry_broadcast_interval_ms: 1000 * 60,
            telemetry_cache_cutoff_ms: 1000 * 130, //  2 * `telemetry_broadcast_interval` + some margin
            optimistic_activation_delay: Duration::from_secs(30),
        }
    }

    fn beta(work: WorkThresholds) -> Self {
        Self {
            current_network: Networks::NanoBetaNetwork,
            default_node_port: 54000,
            default_rpc_port: 55000,
            default_ipc_port: 56000,
            default_websocket_port: 57000,
            ..Self::live(work)
        }
    }

    fn test(work: WorkThresholds) -> Self {
        Self {
            current_network: Networks::NanoTestNetwork,
            default_node_port: test_node_port(),
            default_rpc_port: test_rpc_port(),
            default_ipc_port: test_ipc_port(),
            default_websocket_port: test_websocket_port(),
            ..Self::live(work)
        }
    }

    fn dev(work: WorkThresholds) -> Self {
        let cleanup_period_s = 1;
        let max_peers_per_ip = 20;
        Self {
            current_network: Networks::NanoDevNetwork,
            default_node_port: 44000,
            default_rpc_port: 45000,
            default_ipc_port: 46000,
            default_websocket_port: 47000,
            aec_loop_interval_ms: 20,
            cleanup_period_s,
            keepalive_period: Duration::from_secs(1),
            idle_timeout_s: cleanup_period_s * 15,
            max_peers_per_ip,
            max_peers_per_subnetwork: max_peers_per_ip * 4,
            peer_dump_interval_s: 1,
            vote_broadcast_interval_ms: 500,
            telemetry_request_cooldown_ms: 500,
            telemetry_cache_cutoff_ms: 2000,
            telemetry_request_interval_ms: 500,
            telemetry_broadcast_interval_ms: 500,
            optimistic_activation_delay: Duration::from_secs(2),
            ..Self::live(work)
        }
    }

    pub fn is_live_network(&self) -> bool {
        self.current_network == Networks::NanoLiveNetwork
    }

    pub fn is_beta_network(&self) -> bool {
        self.current_network == Networks::NanoBetaNetwork
    }

    pub fn is_dev_network(&self) -> bool {
        self.current_network == Networks::NanoDevNetwork
    }

    pub fn is_test_network(&self) -> bool {
        self.current_network == Networks::NanoTestNetwork
    }

    /** Initial value is ACTIVE_NETWORK compile flag, but can be overridden by a CLI flag */
    pub fn active_network() -> Networks {
        *ACTIVE_NETWORK.lock().unwrap()
    }

    /**
     * Optionally called on startup to override the global active network.
     * If not called, the compile-time option will be used.
     * @param network The new active network
     */
    pub fn set_active_network(network: Networks) {
        *ACTIVE_NETWORK.lock().unwrap() = network;
    }

    /**
     * Optionally called on startup to override the global active network.
     * If not called, the compile-time option will be used.
     * @param network The new active network. Valid values are "live", "beta" and "dev"
     */
    pub fn set_active_network_from_str(network: impl AsRef<str>) -> Result<()> {
        let net = match network.as_ref() {
            "live" => Networks::NanoLiveNetwork,
            "beta" => Networks::NanoBetaNetwork,
            "dev" => Networks::NanoDevNetwork,
            "test" => Networks::NanoTestNetwork,
            _ => bail!("invalid network"),
        };
        Self::set_active_network(net);
        Ok(())
    }

    pub fn cleanup_period_half_ms(&self) -> i64 {
        (self.cleanup_period_s * 1000) / 2
    }

    pub fn cleanup_cutoff_s(&self) -> i64 {
        self.cleanup_period_s * 5
    }

    pub fn get_current_network_as_string(&self) -> &str {
        match self.current_network {
            Networks::NanoDevNetwork => "dev",
            Networks::NanoBetaNetwork => "beta",
            Networks::NanoLiveNetwork => "live",
            Networks::NanoTestNetwork => "test",
            Networks::Invalid => panic!("invalid network"),
        }
    }
}
pub fn test_node_port() -> u16 {
    get_env_or_default("NANO_TEST_NODE_PORT", 17075)
}

fn test_rpc_port() -> u16 {
    get_env_or_default("NANO_TEST_RPC_PORT", 17076)
}

fn test_ipc_port() -> u16 {
    get_env_or_default("NANO_TEST_IPC_PORT", 17077)
}

fn test_websocket_port() -> u16 {
    get_env_or_default("NANO_TEST_WEBSOCKET_PORT", 17078)
}