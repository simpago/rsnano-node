use crate::{
    IpcConfig, IpcConfigDomainSocket, IpcConfigFlatbuffers, IpcConfigTcpSocket, IpcConfigTransport,
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
    pub io_timeout: Option<usize>,
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

impl From<&IpcConfigToml> for IpcConfig {
    fn from(toml: &IpcConfigToml) -> Self {
        let mut config = IpcConfig::default();

        if let Some(transport_domain) = &toml.transport_domain {
            config.transport_domain = transport_domain.into();
        }
        if let Some(transport_tcp) = &toml.transport_tcp {
            config.transport_tcp = transport_tcp.into();
        }
        if let Some(flatbuffers) = &toml.flatbuffers {
            config.flatbuffers = flatbuffers.into();
        }
        config
    }
}

impl From<&IpcConfigDomainSocketToml> for IpcConfigDomainSocket {
    fn from(toml: &IpcConfigDomainSocketToml) -> Self {
        let mut config = IpcConfigDomainSocket::new();

        if let Some(transport) = &toml.transport {
            config.transport = transport.into();
        }
        if let Some(path) = &toml.path {
            config.path = path.clone();
        }
        config
    }
}

impl From<&IpcConfigTcpSocketToml> for IpcConfigTcpSocket {
    fn from(toml: &IpcConfigTcpSocketToml) -> Self {
        let mut config = IpcConfigTcpSocket::default();

        if let Some(transport) = &toml.transport {
            config.transport = transport.into();
        }
        if let Some(port) = toml.port {
            config.port = port;
        }
        config
    }
}

impl From<&IpcConfigFlatbuffersToml> for IpcConfigFlatbuffers {
    fn from(toml: &IpcConfigFlatbuffersToml) -> Self {
        let mut config = IpcConfigFlatbuffers::new();

        if let Some(skip_unexpected_fields_in_json) = toml.skip_unexpected_fields_in_json {
            config.skip_unexpected_fields_in_json = skip_unexpected_fields_in_json;
        }
        if let Some(verify_buffers) = toml.verify_buffers {
            config.verify_buffers = verify_buffers;
        }
        config
    }
}

impl From<&IpcConfigTransportToml> for IpcConfigTransport {
    fn from(toml: &IpcConfigTransportToml) -> Self {
        let mut config = IpcConfigTransport::new();

        if let Some(enabled) = toml.enabled {
            config.enabled = enabled;
        }
        if let Some(io_timeout) = toml.io_timeout {
            config.io_timeout = io_timeout;
        }
        config
    }
}
