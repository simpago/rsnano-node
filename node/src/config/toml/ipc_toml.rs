use crate::{IpcConfig, IpcConfigDomainSocket, IpcConfigFlatbuffers, IpcConfigTcpSocket};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Deserialize, Serialize)]
pub struct IpcToml {
    pub flatbuffers: Option<FlatbuffersToml>,
    pub local: Option<LocalToml>,
    pub tcp: Option<TcpToml>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct LocalToml {
    pub allow_unsafe: Option<bool>,
    pub enable: Option<bool>,
    pub io_timeout: Option<usize>,
    pub io_threads: Option<i64>,
    pub path: Option<PathBuf>,
}

impl Default for LocalToml {
    fn default() -> Self {
        let config = IpcConfigDomainSocket::default();
        (&config).into()
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct FlatbuffersToml {
    pub skip_unexpected_fields_in_json: Option<bool>,
    pub verify_buffers: Option<bool>,
}

impl Default for FlatbuffersToml {
    fn default() -> Self {
        let config = IpcConfigFlatbuffers::default();
        (&config).into()
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct TcpToml {
    pub enable: Option<bool>,
    pub io_timeout: Option<usize>,
    pub io_threads: Option<i64>,
    pub port: Option<u16>,
}

impl From<&IpcConfig> for IpcToml {
    fn from(config: &IpcConfig) -> Self {
        Self {
            local: Some((&config.transport_domain).into()),
            flatbuffers: Some((&config.flatbuffers).into()),
            tcp: Some((&config.transport_tcp).into()),
        }
    }
}

impl IpcConfig {
    pub fn merge_toml(&mut self, toml: &IpcToml) {
        if let Some(transport_domain) = &toml.tcp {
            self.transport_tcp.merge_toml(transport_domain);
        }
        if let Some(transport_tcp) = &toml.local {
            self.transport_domain = transport_tcp.into();
        }
        if let Some(flatbuffers) = &toml.flatbuffers {
            self.flatbuffers = flatbuffers.into();
        }
    }
}

impl From<&LocalToml> for IpcConfigDomainSocket {
    fn from(toml: &LocalToml) -> Self {
        let mut config = IpcConfigDomainSocket::default();

        if let Some(enable) = toml.enable {
            config.transport.enabled = enable;
        }
        if let Some(path) = &toml.path {
            config.path = path.clone();
        }
        if let Some(io_timeout) = toml.io_timeout {
            config.transport.io_timeout = io_timeout;
        }
        if let Some(io_threads) = toml.io_threads {
            config.transport.io_threads = io_threads;
        }
        if let Some(allow_unsafe) = toml.allow_unsafe {
            config.transport.allow_unsafe = allow_unsafe;
        }
        config
    }
}

impl From<&IpcConfigDomainSocket> for LocalToml {
    fn from(config: &IpcConfigDomainSocket) -> Self {
        Self {
            enable: Some(config.transport.enabled),
            io_timeout: Some(config.transport.io_timeout),
            io_threads: Some(config.transport.io_threads),
            path: Some(config.path.clone()),
            allow_unsafe: Some(config.transport.allow_unsafe),
        }
    }
}

impl From<&FlatbuffersToml> for IpcConfigFlatbuffers {
    fn from(toml: &FlatbuffersToml) -> Self {
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

impl From<&IpcConfigFlatbuffers> for FlatbuffersToml {
    fn from(config: &IpcConfigFlatbuffers) -> Self {
        Self {
            skip_unexpected_fields_in_json: Some(config.skip_unexpected_fields_in_json),
            verify_buffers: Some(config.verify_buffers),
        }
    }
}

impl IpcConfigTcpSocket {
    pub fn merge_toml(&mut self, toml: &TcpToml) {
        if let Some(enable) = toml.enable {
            self.transport.enabled = enable;
        }
        if let Some(io_timeout) = toml.io_timeout {
            self.transport.io_timeout = io_timeout;
        }
        if let Some(io_threads) = toml.io_threads {
            self.transport.io_threads = io_threads;
        }
        if let Some(port) = toml.port {
            self.port = port;
        }
    }
}

impl From<&IpcConfigTcpSocket> for TcpToml {
    fn from(config: &IpcConfigTcpSocket) -> Self {
        Self {
            enable: Some(config.transport.enabled),
            io_timeout: Some(config.transport.io_timeout),
            io_threads: Some(config.transport.io_threads),
            port: Some(config.port),
        }
    }
}
