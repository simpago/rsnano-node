use super::{RpcServerConfig, RpcServerLoggingConfig, RpcServerProcessConfig};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct RpcServerToml {
    pub address: Option<String>,
    pub enable_control: Option<bool>,
    pub max_json_depth: Option<u8>,
    pub max_request_size: Option<u64>,
    pub port: Option<u16>,
    pub logging: Option<RpcServerLoggingToml>,
    pub process: Option<RpcServerProcessToml>,
}

impl From<&RpcServerConfig> for RpcServerToml {
    fn from(config: &RpcServerConfig) -> Self {
        Self {
            address: Some(config.address.clone()),
            port: Some(config.port),
            enable_control: Some(config.enable_control),
            max_json_depth: Some(config.max_json_depth),
            max_request_size: Some(config.max_request_size),
            logging: Some((&config.rpc_logging).into()),
            process: Some((&config.rpc_process).into()),
        }
    }
}

impl RpcServerConfig {
    pub fn merge_toml(&mut self, toml: &RpcServerToml) {
        if let Some(address) = &toml.address {
            self.address = address.clone();
        }
        if let Some(port) = toml.port {
            self.port = port;
        }
        if let Some(enable_control) = toml.enable_control {
            self.enable_control = enable_control;
        }
        if let Some(max_json_depth) = toml.max_json_depth {
            self.max_json_depth = max_json_depth;
        }
        if let Some(max_request_size) = toml.max_request_size {
            self.max_request_size = max_request_size;
        }
        if let Some(logging) = &toml.logging {
            self.rpc_logging = logging.into();
        }
        if let Some(process) = &toml.process {
            self.rpc_process.merge_toml(process);
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct RpcServerLoggingToml {
    pub log_rpc: Option<bool>,
}

impl Default for RpcServerLoggingToml {
    fn default() -> Self {
        let config = RpcServerLoggingConfig::default();
        (&config).into()
    }
}

impl From<&RpcServerLoggingConfig> for RpcServerLoggingToml {
    fn from(config: &RpcServerLoggingConfig) -> Self {
        Self {
            log_rpc: Some(config.log_rpc),
        }
    }
}

impl From<&RpcServerLoggingToml> for RpcServerLoggingConfig {
    fn from(toml: &RpcServerLoggingToml) -> Self {
        let mut config = RpcServerLoggingConfig::default();
        if let Some(log_rpc) = toml.log_rpc {
            config.log_rpc = log_rpc;
        }
        config
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct RpcServerProcessToml {
    pub io_threads: Option<u32>,
    pub ipc_address: Option<String>,
    pub ipc_port: Option<u16>,
    pub num_ipc_connections: Option<u32>,
}

impl From<&RpcServerProcessConfig> for RpcServerProcessToml {
    fn from(config: &RpcServerProcessConfig) -> Self {
        Self {
            io_threads: Some(config.io_threads),
            ipc_address: Some(config.ipc_address.clone()),
            ipc_port: Some(config.ipc_port),
            num_ipc_connections: Some(config.num_ipc_connections),
        }
    }
}

impl RpcServerProcessConfig {
    pub(crate) fn merge_toml(&mut self, toml: &RpcServerProcessToml) {
        if let Some(io_threads) = toml.io_threads {
            self.io_threads = io_threads;
        }
        if let Some(ipc_address) = &toml.ipc_address {
            self.ipc_address = ipc_address.clone();
        }
        if let Some(ipc_port) = toml.ipc_port {
            self.ipc_port = ipc_port;
        }
        if let Some(num_ipc_connections) = toml.num_ipc_connections {
            self.num_ipc_connections = num_ipc_connections;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{RpcServerConfig, RpcServerToml};
    use rsnano_core::Networks;
    use rsnano_node::config::NetworkConstants;
    use toml::{from_str, to_string};

    static DEFAULT_TOML_STR: &str = r#"
        address = "::1"
        enable_control = false
    	max_json_depth = 20
    	max_request_size = 33554432
        port = 55000

        [logging]
        log_rpc = true

        [process]
    	io_threads = 8
    	ipc_address = "::1"
    	ipc_port = 56000
    	num_ipc_connections = 4"#;

    static MODIFIED_TOML_STR: &str = r#"
        address = "0:0:0:0:0:ffff:7f01:101"
    	enable_control = true
    	max_json_depth = 9
    	max_request_size = 999
    	port = 999

        [logging]
        log_rpc = false

    	[process]
    	io_threads = 999
    	ipc_address = "0:0:0:0:0:ffff:7f01:101"
    	ipc_port = 999
    	num_ipc_connections = 999"#;

    #[test]
    fn deserialize_defaults() {
        let deserialized_toml: RpcServerToml = toml::from_str(&DEFAULT_TOML_STR).unwrap();

        let default_rpc_config = RpcServerConfig::new(&NetworkConstants::for_beta(), 8);
        let mut deserialized_rpc_config = default_rpc_config.clone();
        deserialized_rpc_config.merge_toml(&deserialized_toml);

        assert_eq!(&deserialized_rpc_config, &default_rpc_config);
    }

    #[test]
    fn deserialize_no_defaults() {
        let rpc_toml: RpcServerToml =
            from_str(MODIFIED_TOML_STR).expect("Failed to deserialize TOML");

        let mut deserialized_rpc_config = RpcServerConfig::new(&NetworkConstants::for_beta(), 8);
        deserialized_rpc_config.merge_toml(&rpc_toml);

        let default_rpc_config = RpcServerConfig::new(&NetworkConstants::for_beta(), 8);

        assert_ne!(deserialized_rpc_config.address, default_rpc_config.address);
        assert_ne!(
            deserialized_rpc_config.enable_control,
            default_rpc_config.enable_control
        );
        assert_ne!(
            deserialized_rpc_config.max_json_depth,
            default_rpc_config.max_json_depth
        );
        assert_ne!(
            deserialized_rpc_config.max_request_size,
            default_rpc_config.max_request_size
        );
        assert_ne!(deserialized_rpc_config.port, default_rpc_config.port);

        assert_ne!(
            deserialized_rpc_config.rpc_logging.log_rpc,
            default_rpc_config.rpc_logging.log_rpc
        );

        assert_ne!(
            deserialized_rpc_config.rpc_process.io_threads,
            default_rpc_config.rpc_process.io_threads
        );
        assert_ne!(
            deserialized_rpc_config.rpc_process.ipc_address,
            default_rpc_config.rpc_process.ipc_address
        );
        assert_ne!(
            deserialized_rpc_config.rpc_process.ipc_port,
            default_rpc_config.rpc_process.ipc_port
        );
        assert_ne!(
            deserialized_rpc_config.rpc_process.num_ipc_connections,
            default_rpc_config.rpc_process.num_ipc_connections
        );
    }

    #[test]
    fn deserialize_empty() {
        let toml_str = "";

        let rpc_toml: RpcServerToml = from_str(&toml_str).expect("Failed to deserialize TOML");

        let mut deserialized_rpc_config =
            RpcServerConfig::default_for(Networks::NanoBetaNetwork, 8);
        deserialized_rpc_config.merge_toml(&rpc_toml);

        let default_rpc_config = RpcServerConfig::default_for(Networks::NanoBetaNetwork, 8);

        assert_eq!(&deserialized_rpc_config, &default_rpc_config);
    }

    #[test]
    fn serialize_defaults() {
        let default_rpc_config = RpcServerConfig::default_for(Networks::NanoBetaNetwork, 8);

        let default_rpc_toml: RpcServerToml = (&default_rpc_config).into();

        let serialized_toml = to_string(&default_rpc_toml).unwrap();

        let default_toml_str_trimmed: String = DEFAULT_TOML_STR
            .lines()
            .map(|line| line.trim())
            .collect::<Vec<&str>>()
            .join("\n")
            .trim()
            .to_string();

        let serialized_toml_trimmed: String = serialized_toml
            .lines()
            .map(|line| line.trim())
            .collect::<Vec<&str>>()
            .join("\n")
            .trim()
            .to_string();

        assert_eq!(&serialized_toml_trimmed, &default_toml_str_trimmed);
    }
}
