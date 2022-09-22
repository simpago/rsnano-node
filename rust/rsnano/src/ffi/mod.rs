mod bandwidth_limiter;
mod blake2b;
mod block_arrival;
mod block_processor;
mod blocks;
mod bootstrap;
mod config;
mod epoch;
mod hardened_constants;
mod ipc;
mod logger_mt;
mod numbers;
mod property_tree;
mod secure;
mod signatures;
mod state_block_signature_verification;
mod stats;
mod stream;
mod toml;
mod unchecked_info;
mod voting;
mod websocket;
mod rep_weights;

use std::{ffi::CString, os::raw::c_char};

pub use bandwidth_limiter::*;
pub use blake2b::*;
pub(crate) use block_processor::*;
pub use blocks::*;
pub use config::*;
pub use epoch::*;
pub use ipc::*;
pub use logger_mt::*;
pub use numbers::*;
pub use property_tree::*;
pub use secure::*;
pub use signatures::*;
pub use stats::*;
pub use stream::*;
pub use toml::*;
pub(crate) use unchecked_info::*;
pub(crate) use websocket::*;

use crate::{Account, BlockHash, RawKey, Root};

pub struct StringHandle(CString);
#[repr(C)]
pub struct StringDto {
    pub handle: *mut StringHandle,
    pub value: *const c_char,
}

impl From<String> for StringDto {
    fn from(s: String) -> Self {
        let handle = Box::new(StringHandle(CString::new(s).unwrap()));
        let value = handle.0.as_ptr();
        StringDto {
            handle: Box::into_raw(handle),
            value,
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_string_destroy(handle: *mut StringHandle) {
    drop(Box::from_raw(handle))
}

impl From<*const u8> for BlockHash {
    fn from(ptr: *const u8) -> Self {
        BlockHash::from_bytes(into_32_byte_array(ptr))
    }
}

impl From<*const u8> for Account {
    fn from(ptr: *const u8) -> Self {
        Account::from_bytes(into_32_byte_array(ptr))
    }
}

impl From<*const u8> for Root {
    fn from(ptr: *const u8) -> Self {
        Root::from_bytes(into_32_byte_array(ptr))
    }
}

impl From<*const u8> for RawKey {
    fn from(ptr: *const u8) -> Self {
        RawKey::from_bytes(into_32_byte_array(ptr))
    }
}

fn into_32_byte_array(ptr: *const u8) -> [u8; 32] {
    let mut bytes = [0; 32];
    bytes.copy_from_slice(unsafe { std::slice::from_raw_parts(ptr, 32) });
    bytes
}
