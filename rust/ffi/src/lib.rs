#![allow(clippy::missing_safety_doc)]

#[macro_use]
extern crate anyhow;

#[macro_use]
extern crate num_derive;

pub mod block_processing;
pub mod bootstrap;
mod cementation;
mod config;
mod consensus;
pub mod core;
mod hardened_constants;
mod ipc;
pub mod ledger;
pub mod messages;
mod node;
mod property_tree;
pub mod representatives;
mod secure;
mod stats;
mod telemetry;
mod transport;
mod utils;
mod wallets;
mod websocket;
mod work;

pub use config::*;
pub use ipc::*;
pub use property_tree::*;
use rsnano_core::utils::{IS_SANITIZER_BUILD, MEMORY_INTENSIVE_INSTRUMENTATION};
use std::{
    ffi::{c_void, CStr, CString},
    ops::Deref,
    os::raw::c_char,
};
pub type MemoryIntensiveInstrumentationCallback = extern "C" fn() -> bool;
use rsnano_node::utils::ErrorCode;
pub use secure::*;
pub use stats::*;

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
    IS_SANITIZER_BUILD = f;
}

pub struct U256ArrayHandle {
    _data: Vec<[u8; 32]>,
}

#[repr(C)]
pub struct U256ArrayDto {
    pub items: *const [u8; 32],
    pub count: usize,
    pub handle: *mut U256ArrayHandle,
}

impl U256ArrayDto {
    pub fn initialize(&mut self, values: Vec<[u8; 32]>) {
        self.items = values.as_ptr();
        self.count = values.len();
        self.handle = Box::into_raw(Box::new(U256ArrayHandle { _data: values }))
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_u256_array_destroy(dto: *mut U256ArrayDto) {
    drop(Box::from_raw((*dto).handle))
}

pub(crate) unsafe fn to_rust_string(s: *const c_char) -> String {
    CStr::from_ptr(s).to_str().unwrap().to_owned()
}

pub struct StringVecHandle(Vec<String>);

impl Deref for StringVecHandle {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub extern "C" fn rsn_string_vec_create() -> *mut StringVecHandle {
    Box::into_raw(Box::new(StringVecHandle(Vec::new())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_string_vec_destroy(handle: *mut StringVecHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_string_vec_push(handle: &mut StringVecHandle, value: *const c_char) {
    handle.0.push(to_rust_string(value));
}
