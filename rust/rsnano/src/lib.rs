#![allow(clippy::missing_safety_doc)]
#![allow(dead_code)]

#[macro_use]
extern crate static_assertions;

#[macro_use]
extern crate num_derive;

#[macro_use]
extern crate anyhow;

pub mod block_processing;
pub mod bootstrap;
pub mod config;
pub mod core;
pub mod ffi;
mod ipc;
pub mod ledger;
mod secure;
pub mod signatures;
pub mod stats;
pub mod transport;
pub mod utils;
pub mod voting;
pub mod wallet;
mod websocket;
pub mod work;
mod election_status;

pub use ipc::*;
pub use secure::*;
pub(crate) use websocket::*;
pub use election_status::*;

pub type MemoryIntensiveInstrumentationCallback = extern "C" fn() -> bool;

pub static mut MEMORY_INTENSIVE_INSTRUMENTATION: Option<MemoryIntensiveInstrumentationCallback> =
    None;
pub static mut IS_SANITIZER_BUILD: Option<MemoryIntensiveInstrumentationCallback> = None;

pub fn memory_intensive_instrumentation() -> bool {
    unsafe {
        match MEMORY_INTENSIVE_INSTRUMENTATION {
            Some(f) => f(),
            None => false,
        }
    }
}

pub fn is_sanitizer_build() -> bool {
    unsafe { IS_SANITIZER_BUILD.expect("IS_SANITIZER_BUILD missing")() }
}
