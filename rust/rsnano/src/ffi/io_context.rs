use super::thread_pool::VoidFnCallbackHandle;
use crate::utils::IoContext;
use std::ffi::c_void;

pub struct IoContextHandle(*mut c_void);

impl IoContextHandle {
    /// raw_handle is a `boost::asio::io_context *`
    pub fn new(raw_handle: *mut c_void) -> *mut IoContextHandle {
        Box::into_raw(Box::new(IoContextHandle(raw_handle)))
    }

    pub fn raw_handle(&self) -> *mut c_void {
        self.0
    }
}

/// handle is a `boost::asio::io_context *`
#[no_mangle]
pub extern "C" fn rsn_io_ctx_create(handle: *mut c_void) -> *mut IoContextHandle {
    Box::into_raw(Box::new(IoContextHandle(handle)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_io_ctx_destroy(handle: *mut IoContextHandle) {
    drop(Box::from_raw(handle));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_io_ctx_get_ctx(handle: *mut IoContextHandle) -> *mut c_void {
    (*handle).0
}

pub type DispatchCallback = unsafe extern "C" fn(*mut c_void, *mut VoidFnCallbackHandle);

static mut POST_CALLBACK: Option<DispatchCallback> = None;

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_io_ctx_post(f: DispatchCallback) {
    POST_CALLBACK = Some(f);
}

pub struct FfiIoContext {
    /// handle is a `boost::asio::io_context *`
    handle: *mut c_void,
}

impl FfiIoContext {
    pub fn new(handle: *mut c_void) -> Self {
        Self { handle }
    }
}

impl IoContext for FfiIoContext {
    fn post(&self, f: Box<dyn FnOnce()>) {
        unsafe {
            POST_CALLBACK.expect("POST_CALLBACK missing")(
                self.handle,
                Box::into_raw(Box::new(VoidFnCallbackHandle::new(f))),
            );
        }
    }

    fn raw_handle(&self) -> *mut c_void {
        self.handle
    }
}
