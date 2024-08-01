mod active_elections_toml;
mod block_processor_toml;
mod bootstrap_ascending_toml;
mod bootstrap_server_toml;
mod daemon_toml;
mod diagnostics_toml;
mod ipc_toml;
mod lmdb_toml;
mod message_processor_toml;
mod monitor_toml;
mod node_rpc_toml;
mod node_toml;
mod opencl_toml;
mod optimistic_scheduler_toml;
mod priority_bucket_toml;
mod request_aggregator_toml;
mod rpc_toml;
mod stats_toml;
mod vote_cache_toml;
mod vote_processor_toml;
mod websocket_toml;

pub use active_elections_toml::*;
pub use block_processor_toml::*;
pub use bootstrap_ascending_toml::*;
pub use bootstrap_server_toml::*;
pub use daemon_toml::*;
pub use diagnostics_toml::*;
pub use ipc_toml::*;
pub use lmdb_toml::*;
pub use message_processor_toml::*;
pub use monitor_toml::*;
pub use node_rpc_toml::*;
pub use node_toml::*;
pub use opencl_toml::*;
pub use optimistic_scheduler_toml::*;
pub use priority_bucket_toml::*;
pub use request_aggregator_toml::*;
pub use rpc_toml::*;
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
pub use stats_toml::*;
pub use vote_cache_toml::*;
pub use vote_processor_toml::*;
pub use websocket_toml::*;

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
