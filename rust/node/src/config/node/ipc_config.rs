use crate::{
    config::TomlConfigOverride, IpcConfig, IpcConfigDomainSocket, IpcConfigFlatbuffers,
    IpcConfigTcpSocket, IpcConfigTransport,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Serialize)]
pub struct IpcConfigToml {
    pub transport_domain: Option<IpcConfigDomainSocketToml>,
    pub transport_tcp: Option<IpcConfigTcpSocketToml>,
    pub flatbuffers: Option<IpcConfigFlatbuffersToml>,
}

#[derive(Deserialize, Serialize)]
pub struct IpcConfigDomainSocketToml {
    pub transport: Option<IpcConfigTransportToml>,
    pub path: Option<PathBuf>,
}

#[derive(Deserialize, Serialize)]
pub struct IpcConfigTransportToml {
    pub enabled: Option<bool>,
    //pub allow_unsafe: Option<bool>,
    pub io_timeout: Option<usize>,
    //pub io_threads: Option<i64>,
}

#[derive(Deserialize, Serialize)]
pub struct IpcConfigFlatbuffersToml {
    pub skip_unexpected_fields_in_json: Option<bool>,
    pub verify_buffers: Option<bool>,
}

#[derive(Deserialize, Serialize)]
pub struct IpcConfigTcpSocketToml {
    pub transport: Option<IpcConfigTransportToml>,
    pub port: Option<u16>,
}

impl From<IpcConfig> for IpcConfigToml {
    fn from(config: IpcConfig) -> Self {
        Self {
            transport_domain: Some(IpcConfigDomainSocketToml {
                transport: Some(IpcConfigTransportToml {
                    enabled: Some(config.transport_domain.transport.enabled),
                    //allow_unsafe: Some(config.transport_domain.transport.allow_unsafe),
                    io_timeout: Some(config.transport_domain.transport.io_timeout),
                    //io_threads: Some(config.transport_domain.transport.io_threads),
                }),
                path: Some(config.transport_domain.path),
            }),
            transport_tcp: Some(IpcConfigTcpSocketToml {
                transport: Some(IpcConfigTransportToml {
                    enabled: Some(config.transport_tcp.transport.enabled),
                    //allow_unsafe: Some(config.transport_tcp.transport.allow_unsafe),
                    io_timeout: Some(config.transport_tcp.transport.io_timeout),
                    //io_threads: Some(config.transport_tcp.transport.io_threads),
                }),
                port: Some(config.transport_tcp.port),
            }),
            flatbuffers: Some(IpcConfigFlatbuffersToml {
                skip_unexpected_fields_in_json: Some(
                    config.flatbuffers.skip_unexpected_fields_in_json,
                ),
                verify_buffers: Some(config.flatbuffers.verify_buffers),
            }),
        }
    }
}

impl<'de> TomlConfigOverride<'de, IpcConfigToml> for IpcConfig {
    fn toml_config_override(&mut self, toml: &'de IpcConfigToml) {
        if let Some(transport_domain) = &toml.transport_domain {
            self.transport_domain.toml_config_override(transport_domain);
        }
        if let Some(transport_tcp) = &toml.transport_tcp {
            self.transport_tcp.toml_config_override(transport_tcp);
        }
        if let Some(flatbuffers) = &toml.flatbuffers {
            self.flatbuffers.toml_config_override(flatbuffers);
        }
    }
}

impl<'de> TomlConfigOverride<'de, IpcConfigDomainSocketToml> for IpcConfigDomainSocket {
    fn toml_config_override(&mut self, toml: &'de IpcConfigDomainSocketToml) {
        if let Some(transport) = &toml.transport {
            self.transport.toml_config_override(transport);
        }
        if let Some(path) = &toml.path {
            self.path = path.clone();
        }
    }
}

impl<'de> TomlConfigOverride<'de, IpcConfigTcpSocketToml> for IpcConfigTcpSocket {
    fn toml_config_override(&mut self, toml: &'de IpcConfigTcpSocketToml) {
        if let Some(transport) = &toml.transport {
            self.transport.toml_config_override(transport);
        }
        if let Some(port) = toml.port {
            self.port = port;
        }
    }
}

impl<'de> TomlConfigOverride<'de, IpcConfigFlatbuffersToml> for IpcConfigFlatbuffers {
    fn toml_config_override(&mut self, toml: &'de IpcConfigFlatbuffersToml) {
        if let Some(skip_unexpected_fields_in_json) = toml.skip_unexpected_fields_in_json {
            self.skip_unexpected_fields_in_json = skip_unexpected_fields_in_json;
        }
        if let Some(verify_buffers) = toml.verify_buffers {
            self.verify_buffers = verify_buffers;
        }
    }
}

impl<'de> TomlConfigOverride<'de, IpcConfigTransportToml> for IpcConfigTransport {
    fn toml_config_override(&mut self, toml: &'de IpcConfigTransportToml) {
        if let Some(enabled) = toml.enabled {
            self.enabled = enabled;
        }
        //if let Some(allow_unsafe) = toml.allow_unsafe {
        //self.allow_unsafe = allow_unsafe;
        //}
        if let Some(io_timeout) = toml.io_timeout {
            self.io_timeout = io_timeout;
        }
        //if let Some(io_threads) = toml.io_threads {
        //self.io_threads = io_threads;
        //}
    }
}
