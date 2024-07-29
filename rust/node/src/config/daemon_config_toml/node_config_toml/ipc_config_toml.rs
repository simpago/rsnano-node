use crate::IpcConfig;
use anyhow::Result;
use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Serialize)]
pub struct IpcConfigToml {
    pub transport_domain: Option<TomlIpcConfigDomainSocket>,
    pub transport_tcp: Option<IpcConfigTcpSocketToml>,
    pub flatbuffers: Option<TomlIpcConfigFlatbuffers>,
}

impl IpcConfigToml {
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

#[derive(Deserialize, Serialize)]
pub struct TomlIpcConfigDomainSocket {
    pub transport: Option<TomlIpcConfigTransport>,
    pub path: Option<PathBuf>,
}

#[derive(Deserialize, Serialize)]
pub struct TomlIpcConfigTransport {
    pub enabled: Option<bool>,
    //pub allow_unsafe: Option<bool>,
    pub io_timeout: Option<usize>,
    //pub io_threads: Option<i64>,
}

#[derive(Deserialize, Serialize)]
pub struct TomlIpcConfigFlatbuffers {
    pub skip_unexpected_fields_in_json: Option<bool>,
    pub verify_buffers: Option<bool>,
}

#[derive(Deserialize, Serialize)]
pub struct IpcConfigTcpSocketToml {
    pub transport: Option<TomlIpcConfigTransport>,
    pub port: Option<u16>,
}

impl Default for IpcConfigToml {
    fn default() -> Self {
        let config = IpcConfig::default();
        Self {
            transport_domain: Some(config.transport_domain),
            transport_tcp: Some(config.transport_tcp),
            flatbuffers: Some(config.flatbuffers),
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
