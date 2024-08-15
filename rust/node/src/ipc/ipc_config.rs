use crate::config::NetworkConstants;
use std::path::PathBuf;

/** Base for transport configurations */
#[derive(Clone, Debug, PartialEq)]
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
#[derive(Clone, Debug, PartialEq)]
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
#[derive(Clone, Debug, PartialEq)]
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
#[derive(Clone, Debug, PartialEq)]
pub struct IpcConfigTcpSocket {
    pub transport: IpcConfigTransport,
    pub network_constants: NetworkConstants,
    /** Listening port */
    pub port: u16,
}

impl IpcConfigTcpSocket {
    pub fn new(network_constants: &NetworkConstants) -> Self {
        Self {
            transport: IpcConfigTransport::new(),
            network_constants: network_constants.clone(),
            port: network_constants.default_ipc_port,
        }
    }
}

impl Default for IpcConfigTcpSocket {
    fn default() -> Self {
        Self::new(&NetworkConstants::default())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct IpcConfig {
    pub transport_domain: IpcConfigDomainSocket,
    pub transport_tcp: IpcConfigTcpSocket,
    pub flatbuffers: IpcConfigFlatbuffers,
}

impl IpcConfig {
    pub fn new(network_constants: &NetworkConstants) -> Self {
        Self {
            transport_domain: IpcConfigDomainSocket::new(),
            transport_tcp: IpcConfigTcpSocket::new(network_constants),
            flatbuffers: IpcConfigFlatbuffers::new(),
        }
    }
}

impl Default for IpcConfig {
    fn default() -> Self {
        Self::new(&NetworkConstants::default())
    }
}
