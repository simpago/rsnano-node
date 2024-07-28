use super::toml::{
    IpcConfigDomainSocketToml, IpcConfigFlatbuffersToml, IpcConfigTcpSocketToml, IpcConfigToml,
    IpcConfigTransportToml,
};
use crate::config::NetworkConstants;
use anyhow::Result;
use rsnano_core::utils::TomlWriter;
use std::path::PathBuf;

/** Base for transport configurations */
#[derive(Clone)]
pub struct IpcConfigTransport {
    pub enabled: bool,
    pub allow_unsafe: bool,
    pub io_timeout: usize,
    pub io_threads: i64,
}

impl Default for IpcConfigTransport {
    fn default() -> Self {
        Self {
            enabled: false,
            allow_unsafe: false,
            io_timeout: 15,
            io_threads: -1,
        }
    }
}

impl IpcConfigTransport {
    pub fn new() -> Self {
        Default::default()
    }
}

/**
 * Flatbuffers encoding config. See TOML serialization calls for details about each field.
 */
#[derive(Clone)]
pub struct IpcConfigFlatbuffers {
    pub skip_unexpected_fields_in_json: bool,
    pub verify_buffers: bool,
}

impl Default for IpcConfigFlatbuffers {
    fn default() -> Self {
        Self {
            skip_unexpected_fields_in_json: true,
            verify_buffers: true,
        }
    }
}

impl IpcConfigFlatbuffers {
    pub fn new() -> Self {
        Default::default()
    }
}

/** Domain socket specific transport config */
#[derive(Clone)]
pub struct IpcConfigDomainSocket {
    pub transport: IpcConfigTransport,
    /**
     * Default domain socket path for Unix systems. Once Boost supports Windows 10 usocks,
     * this value will be conditional on OS.
     */
    pub path: PathBuf,
}

impl Default for IpcConfigDomainSocket {
    fn default() -> Self {
        Self {
            transport: IpcConfigTransport::new(),
            path: "/tmp/nano".into(),
        }
    }
}

impl IpcConfigDomainSocket {
    pub fn new() -> Self {
        Default::default()
    }
}

/** TCP specific transport config */
#[derive(Clone)]
pub struct IpcConfigTcpSocket {
    pub transport: IpcConfigTransport,
    /** Listening port */
    pub port: u16,
}

impl Default for IpcConfigTcpSocket {
    fn default() -> Self {
        let network_constants = NetworkConstants::empty();

        Self {
            transport: IpcConfigTransport::new(),
            port: network_constants.default_ipc_port,
        }
    }
}

impl IpcConfigTcpSocket {
    pub fn new(network_constants: &NetworkConstants) -> Self {
        Self {
            transport: IpcConfigTransport::new(),
            port: network_constants.default_ipc_port,
        }
    }
}

#[derive(Clone)]
pub struct IpcConfig {
    pub transport_domain: IpcConfigDomainSocket,
    pub transport_tcp: IpcConfigTcpSocket,
    pub flatbuffers: IpcConfigFlatbuffers,
}

impl Default for IpcConfig {
    fn default() -> Self {
        let network_constants = &NetworkConstants::empty();

        Self {
            transport_domain: IpcConfigDomainSocket::new(),
            transport_tcp: IpcConfigTcpSocket::new(network_constants),
            flatbuffers: IpcConfigFlatbuffers::new(),
        }
    }
}

impl IpcConfig {
    pub fn new(network_constants: &NetworkConstants) -> Self {
        Self {
            transport_domain: IpcConfigDomainSocket::new(),
            transport_tcp: IpcConfigTcpSocket::new(network_constants),
            flatbuffers: IpcConfigFlatbuffers::new(),
        }
    }

    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> Result<()> {
        toml.put_child("tcp", &mut |tcp| {
            tcp.put_bool(
                "enable",
                self.transport_tcp.transport.enabled,
                "Enable or disable IPC via TCP server.\ntype:bool",
            )?;
            tcp.put_u16(
                "port",
                self.transport_tcp.port,
                "Server listening port.\ntype:uint16",
            )?;
            tcp.put_usize(
                "io_timeout",
                self.transport_tcp.transport.io_timeout,
                "Timeout for requests.\ntype:seconds",
            )?;
            // Only write out experimental config values if they're previously set explicitly in the config file
            if self.transport_tcp.transport.io_threads >= 0 {
                tcp.put_i64(
                    "io_threads",
                    self.transport_tcp.transport.io_threads,
                    "Number of threads dedicated to TCP I/O. Experimental.\ntype:uint64_t",
                )?;
            }

            Ok(())
        })?;

        toml.put_child ("local", &mut |domain|{
            if self.transport_domain.transport.io_threads >= 0 {
            	domain.put_i64("io_threads", self.transport_domain.transport.io_threads, "")?;
            }
            domain.put_bool("enable", self.transport_domain.transport.enabled, "Enable or disable IPC via local domain socket.\ntype:bool")?;
            domain.put_bool("allow_unsafe", self.transport_domain.transport.allow_unsafe, "If enabled, certain unsafe RPCs can be used. Not recommended for production systems.\ntype:bool")?;
            domain.put_str("path", self.transport_domain.path.to_str().unwrap_or(""), "Path to the local domain socket.\ntype:string")?;
            domain.put_usize("io_timeout", self.transport_domain.transport.io_timeout, "Timeout for requests.\ntype:seconds")?;

            Ok(())
        })?;

        toml.put_child("flatbuffers", &mut |flatbuffers|{
            flatbuffers.put_bool("skip_unexpected_fields_in_json", self.flatbuffers.skip_unexpected_fields_in_json, "Allow client to send unknown fields in json messages. These will be ignored.\ntype:bool")?;
            flatbuffers.put_bool("verify_buffers", self.flatbuffers.verify_buffers, "Verify that the buffer is valid before parsing. This is recommended when receiving data from untrusted sources.\ntype:bool")?;
            Ok(())
        })?;

        Ok(())
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
