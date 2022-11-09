mod blake2b;
mod buffer;
mod io_context;
mod json;
mod logger_mt;
mod stream;
mod thread_pool;
mod toml;

pub use blake2b::*;
pub use buffer::*;
pub use io_context::*;
pub use json::*;
pub use logger_mt::{Logger, NullLogger};
pub use stream::*;
pub use thread_pool::*;
pub use toml::*;

use crate::config::get_env_or_default_string;

pub fn seconds_since_epoch() -> u64 {
    chrono::Utc::now().timestamp() as u64
}

pub fn get_cpu_count() -> usize {
    // Try to read overridden value from environment variable
    let value = get_env_or_default_string("NANO_HARDWARE_CONCURRENCY", "0")
        .parse::<usize>()
        .unwrap_or_default();
    if value > 0 {
        return value;
    }

    //todo: use std::thread::available_concurrency once it's in stable
    if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
        cpuinfo.match_indices("processor").count()
    } else {
        1
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ErrorCode {
    pub val: i32,
    pub category: u8,
}

pub mod error_category {
    pub const GENERIC: u8 = 0;
    pub const SYSTEM: u8 = 1;
}

impl Default for ErrorCode {
    fn default() -> Self {
        Self {
            val: 0,
            category: error_category::SYSTEM,
        }
    }
}

impl ErrorCode {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn is_ok(&self) -> bool {
        !self.is_err()
    }

    pub fn is_err(&self) -> bool {
        self.val != 0
    }

    pub fn not_supported() -> Self {
        ErrorCode {
            val: 95,
            category: error_category::GENERIC,
        }
    }

    pub fn no_buffer_space() -> Self {
        ErrorCode {
            val: 105,
            category: error_category::GENERIC,
        }
    }

    pub fn host_unreachable() -> Self {
        ErrorCode {
            val: 113,
            category: error_category::GENERIC,
        }
    }

    pub fn fault() -> Self {
        ErrorCode {
            val: 14,
            category: error_category::GENERIC,
        }
    }
}

pub trait Serialize {
    fn serialized_size() -> usize;
    fn serialize(&self, stream: &mut dyn Stream) -> anyhow::Result<()>;
}

pub trait Deserialize {
    type Target;
    fn deserialize(stream: &mut dyn Stream) -> anyhow::Result<Self::Target>;
}

impl Serialize for u64 {
    fn serialized_size() -> usize {
        std::mem::size_of::<u64>()
    }

    fn serialize(&self, stream: &mut dyn Stream) -> anyhow::Result<()> {
        stream.write_u64_be(*self)
    }
}

impl Deserialize for u64 {
    type Target = Self;
    fn deserialize(stream: &mut dyn Stream) -> anyhow::Result<u64> {
        stream.read_u64_be()
    }
}
