mod block_processor;
mod bootstrap_ascending;
mod bootstrap_server;
mod daemon;
mod monitor;
mod node;
mod node_rpc;
mod opencl;
mod stats;
mod websocket;

pub use block_processor::*;
pub use bootstrap_ascending::*;
pub use bootstrap_server::*;
pub use daemon::*;
pub use monitor::*;
pub use node::*;
pub use node_rpc::*;
pub use opencl::*;
pub use stats::*;
pub use websocket::*;

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone)]
pub struct Miliseconds(pub(crate) u128);

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
