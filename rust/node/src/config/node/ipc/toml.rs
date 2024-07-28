use super::config::IpcConfig;
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
