mod active_elections_config_toml;
mod block_processor_config_toml;
mod bootstrap_ascending_config_toml;
mod bootstrap_server_config_toml;
mod daemon_config_toml;
mod diagnostics_config_toml;
mod ipc_config_toml;
mod lmdb_config_toml;
mod message_processor_config_toml;
mod monitor_config_toml;
mod node_config_toml;
mod node_rpc_config_toml;
mod opencl_config_toml;
mod optimistic_scheduler_config_toml;
mod priority_bucket_config_toml;
mod request_aggregator_config_toml;
mod stats_config_toml;
mod vote_cache_config_toml;
mod vote_processor_config_toml;
mod websocket_config_toml;

pub use active_elections_config_toml::*;
pub use block_processor_config_toml::*;
pub use bootstrap_ascending_config_toml::*;
pub use bootstrap_server_config_toml::*;
pub use daemon_config_toml::*;
pub use diagnostics_config_toml::*;
pub use ipc_config_toml::*;
pub use lmdb_config_toml::*;
pub use message_processor_config_toml::*;
pub use monitor_config_toml::*;
pub use node_config_toml::*;
pub use node_rpc_config_toml::*;
pub use opencl_config_toml::*;
pub use optimistic_scheduler_config_toml::*;
pub use priority_bucket_config_toml::*;
pub use request_aggregator_config_toml::*;
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
pub use stats_config_toml::*;
pub use vote_cache_config_toml::*;
pub use vote_processor_config_toml::*;
pub use websocket_config_toml::*;

#[derive(Clone, Default)]
pub struct Miliseconds(pub u128);

impl Serialize for Miliseconds {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Miliseconds {
    fn deserialize<D>(deserializer: D) -> Result<Miliseconds, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let miliseconds = s.parse::<u128>().map_err(Error::custom)?;
        Ok(Miliseconds(miliseconds))
    }
}
