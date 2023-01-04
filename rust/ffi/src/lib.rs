#[macro_use]
extern crate anyhow;

pub mod block_processing;
pub mod bootstrap;
mod config;
mod confirmation_height;
pub mod core;
mod hardened_constants;
mod ipc;
pub mod ledger;
pub mod messages;
pub mod online_reps;
mod property_tree;
mod secure;
mod signatures;
mod stats;
mod transport;
mod utils;
mod voting;
mod wallet;
mod websocket;
mod work;

use std::{
    ffi::{c_void, CString},
    os::raw::c_char,
};

pub use config::*;
pub use ipc::*;
pub use property_tree::*;
use rsnano_core::{
    utils::{IS_SANITIZER_BUILD, MEMORY_INTENSIVE_INSTRUMENTATION},
    Account, Amount, BlockHash, HashOrAccount, Link, PublicKey, RawKey, Root, Signature,
};
pub type MemoryIntensiveInstrumentationCallback = extern "C" fn() -> bool;
pub use secure::*;
pub use signatures::*;
pub use stats::*;
pub(crate) use websocket::*;

use rsnano_node::utils::ErrorCode;

pub struct StringHandle(CString);
#[repr(C)]
pub struct StringDto {
    pub handle: *mut StringHandle,
    pub value: *const c_char,
}

impl<T: AsRef<str>> From<T> for StringDto {
    fn from(s: T) -> Self {
        let handle = Box::new(StringHandle(CString::new(s.as_ref()).unwrap()));
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

pub(crate) unsafe fn copy_public_key_bytes(source: &PublicKey, target: *mut u8) {
    let bytes = std::slice::from_raw_parts_mut(target, 32);
    bytes.copy_from_slice(source.as_bytes());
}

pub(crate) unsafe fn copy_raw_key_bytes(source: RawKey, target: *mut u8) {
    let bytes = std::slice::from_raw_parts_mut(target, 32);
    bytes.copy_from_slice(source.as_bytes());
}

pub(crate) unsafe fn copy_hash_bytes(source: BlockHash, target: *mut u8) {
    let bytes = std::slice::from_raw_parts_mut(target, 32);
    bytes.copy_from_slice(source.as_bytes());
}

pub(crate) unsafe fn copy_hash_or_account_bytes(source: HashOrAccount, target: *mut u8) {
    let bytes = std::slice::from_raw_parts_mut(target, 32);
    bytes.copy_from_slice(source.as_bytes());
}

pub(crate) unsafe fn copy_account_bytes(source: Account, target: *mut u8) {
    let bytes = std::slice::from_raw_parts_mut(target, 32);
    bytes.copy_from_slice(source.as_bytes());
}

pub(crate) unsafe fn copy_signature_bytes(source: &Signature, target: *mut u8) {
    let bytes = std::slice::from_raw_parts_mut(target, 64);
    bytes.copy_from_slice(source.as_bytes());
}

pub(crate) unsafe fn copy_amount_bytes(source: Amount, target: *mut u8) {
    let bytes = std::slice::from_raw_parts_mut(target, 16);
    bytes.copy_from_slice(&source.to_be_bytes());
}

pub(crate) unsafe fn copy_root_bytes(source: Root, target: *mut u8) {
    let bytes = std::slice::from_raw_parts_mut(target, 32);
    bytes.copy_from_slice(source.as_bytes());
}

pub(crate) unsafe fn copy_link_bytes(source: Link, target: *mut u8) {
    let bytes = std::slice::from_raw_parts_mut(target, 32);
    bytes.copy_from_slice(source.as_bytes());
}

pub type VoidPointerCallback = unsafe extern "C" fn(*mut c_void);

#[repr(C)]
pub struct ErrorCodeDto {
    pub val: i32,
    pub category: u8,
}

impl From<&ErrorCode> for ErrorCodeDto {
    fn from(ec: &ErrorCode) -> Self {
        Self {
            val: ec.val,
            category: ec.category,
        }
    }
}

impl From<&ErrorCodeDto> for ErrorCode {
    fn from(dto: &ErrorCodeDto) -> Self {
        Self {
            val: dto.val,
            category: dto.category,
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_memory_intensive_instrumentation(
    f: MemoryIntensiveInstrumentationCallback,
) {
    MEMORY_INTENSIVE_INSTRUMENTATION = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_is_sanitizer_build(
    f: MemoryIntensiveInstrumentationCallback,
) {
    IS_SANITIZER_BUILD = Some(f);
}

pub struct U256ArrayHandle(Box<Vec<[u8; 32]>>);

#[repr(C)]
pub struct U256ArrayDto {
    pub items: *const [u8; 32],
    pub count: usize,
    pub handle: *mut U256ArrayHandle,
}

impl U256ArrayDto {
    pub fn initialize(&mut self, values: Box<Vec<[u8; 32]>>) {
        self.items = values.as_ptr();
        self.count = values.len();
        self.handle = Box::into_raw(Box::new(U256ArrayHandle(values)))
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_u256_array_destroy(dto: *mut U256ArrayDto) {
    drop(Box::from_raw((*dto).handle))
}
