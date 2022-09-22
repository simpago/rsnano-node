use std::{ffi::c_void, ops::Deref, sync::Arc, time::Duration};

use crate::{
    bootstrap::{BootstrapClient, BootstrapClientObserver, BootstrapClientObserverWeakPtr},
    ffi::{
        messages::MessageHandle,
        network::{
            as_tcp_channel, ChannelHandle, ChannelTcpSendBufferCallback, ChannelTcpSendCallback,
            ChannelTcpSendCallbackWrapper, EndpointDto, ReadCallbackWrapper,
            SendBufferCallbackWrapper, SocketDestroyContext, SocketHandle, SocketReadCallback,
        },
        StringDto, VoidPointerCallback,
    },
    network::BufferDropPolicy,
};

use num_traits::FromPrimitive;

pub struct BootstrapClientHandle(BootstrapClient);

/// `observer` is a `shared_ptr<bootstrap_client_observer>*`
#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_client_create(
    observer: *mut c_void,
    channel: *mut ChannelHandle,
    socket: *mut SocketHandle,
) -> *mut BootstrapClientHandle {
    let observer = Arc::new(FfiBootstrapClientObserver::new(observer));
    let channel = as_tcp_channel(channel).clone();
    let socket = (*socket).deref().clone();
    Box::into_raw(Box::new(BootstrapClientHandle(BootstrapClient::new(
        observer, channel, socket,
    ))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_client_destroy(handle: *mut BootstrapClientHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_client_socket(
    handle: *mut BootstrapClientHandle,
) -> *mut SocketHandle {
    SocketHandle::new(Arc::clone((*handle).0.get_socket()))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_client_read(
    handle: *mut BootstrapClientHandle,
    size: usize,
    callback: SocketReadCallback,
    destroy_context: SocketDestroyContext,
    context: *mut c_void,
) {
    let cb_wrapper = ReadCallbackWrapper::new(callback, destroy_context, context);
    let cb = Box::new(move |ec, size| {
        cb_wrapper.execute(ec, size);
    });
    (*handle).0.read_async(size, cb)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_client_sample_block_rate(
    handle: *mut BootstrapClientHandle,
) -> f64 {
    (*handle).0.sample_block_rate()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_client_set_start_time(handle: *mut BootstrapClientHandle) {
    (*handle).0.set_start_time()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_client_elapsed_seconds(
    handle: *mut BootstrapClientHandle,
) -> f64 {
    (*handle).0.elapsed().as_secs_f64()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_client_receive_buffer_size(
    handle: *mut BootstrapClientHandle,
) -> usize {
    (*handle).0.receive_buffer_len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_client_receive_buffer(
    handle: *mut BootstrapClientHandle,
    buffer: *mut u8,
    len: usize,
) {
    let buffer = std::slice::from_raw_parts_mut(buffer, len);
    buffer.copy_from_slice(&(*handle).0.receive_buffer());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_client_send_buffer(
    handle: *mut BootstrapClientHandle,
    buffer: *const u8,
    len: usize,
    callback: ChannelTcpSendBufferCallback,
    delete_callback: VoidPointerCallback,
    callback_context: *mut c_void,
    policy: u8,
) {
    let buffer = Arc::new(std::slice::from_raw_parts(buffer, len).to_vec());
    let callback_wrapper =
        SendBufferCallbackWrapper::new(callback, callback_context, delete_callback);
    let cb = Box::new(move |ec, size| {
        callback_wrapper.call(ec, size);
    });
    let policy = BufferDropPolicy::from_u8(policy).unwrap();
    (*handle).0.send_buffer(&buffer, Some(cb), policy);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_client_send(
    handle: *mut BootstrapClientHandle,
    msg: *mut MessageHandle,
    callback: ChannelTcpSendCallback,
    delete_callback: VoidPointerCallback,
    context: *mut c_void,
    policy: u8,
) {
    let callback_wrapper = ChannelTcpSendCallbackWrapper::new(context, callback, delete_callback);
    let callback_box = Box::new(move |ec, size| {
        callback_wrapper.call(ec, size);
    });
    (*handle).0.send(
        (*msg).as_ref(),
        Some(callback_box),
        BufferDropPolicy::from_u8(policy).unwrap(),
    );
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_client_inc_block_count(
    handle: *mut BootstrapClientHandle,
) -> u64 {
    (*handle).0.inc_block_count()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_client_block_count(
    handle: *mut BootstrapClientHandle,
) -> u64 {
    (*handle).0.block_count()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_client_block_rate(
    handle: *mut BootstrapClientHandle,
) -> f64 {
    (*handle).0.block_rate()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_client_close_socket(handle: *mut BootstrapClientHandle) {
    (*handle).0.close_socket();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_client_remote_endpoint(
    handle: *mut BootstrapClientHandle,
    endpoint: *mut EndpointDto,
) {
    let ep = (*handle).0.remote_endpoint();
    *endpoint = EndpointDto::from(&ep);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_client_tcp_endpoint(
    handle: *mut BootstrapClientHandle,
    endpoint: *mut EndpointDto,
) {
    let ep = (*handle).0.tcp_endpoint();
    *endpoint = EndpointDto::from(&ep);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_client_channel_string(
    handle: *mut BootstrapClientHandle,
    result: *mut StringDto,
) {
    *result = StringDto::from((*handle).0.channel_string());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_client_set_timeout(
    handle: *mut BootstrapClientHandle,
    timeout_s: u64,
) {
    (*handle).0.set_timeout(Duration::from_secs(timeout_s));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_client_pending_stop(
    handle: *mut BootstrapClientHandle,
) -> bool {
    (*handle).0.pending_stop()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_client_hard_stop(
    handle: *mut BootstrapClientHandle,
) -> bool {
    (*handle).0.hard_stop()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_client_stop(
    handle: *mut BootstrapClientHandle,
    force: bool,
) {
    (*handle).0.stop(force);
}

struct FfiBootstrapClientObserver {
    /// a `shared_ptr<bootstrap_client_observer>*`
    handle: *mut c_void,
}

impl FfiBootstrapClientObserver {
    fn new(handle: *mut c_void) -> Self {
        Self { handle }
    }
}

impl BootstrapClientObserver for FfiBootstrapClientObserver {
    fn bootstrap_client_closed(&self) {
        unsafe {
            CLIENT_CLOSED.expect("CLIENT_CLOSED missing")(self.handle);
        }
    }

    fn to_weak(&self) -> Box<dyn BootstrapClientObserverWeakPtr> {
        let weak_handle =
            unsafe { OBSERVER_TO_WEAK.expect("OBSERVER_TO_WEAK missing")(self.handle) };
        Box::new(FfiBootstrapClientObserverWeakPtr::new(weak_handle))
    }
}

impl Drop for FfiBootstrapClientObserver {
    fn drop(&mut self) {
        unsafe { DROP_OBSERVER.expect("DROP_OBSERVER missing")(self.handle) }
    }
}

struct FfiBootstrapClientObserverWeakPtr {
    /// a `weak_ptr<bootstrap_client_observer>*`
    handle: *mut c_void,
}

impl FfiBootstrapClientObserverWeakPtr {
    fn new(handle: *mut c_void) -> Self {
        Self { handle }
    }
}

impl BootstrapClientObserverWeakPtr for FfiBootstrapClientObserverWeakPtr {
    fn upgrade(&self) -> Option<Arc<dyn BootstrapClientObserver>> {
        let observer_handle =
            unsafe { WEAK_TO_OBSERVER.expect("WEAK_TO_OBSERVER missing")(self.handle) };
        if observer_handle.is_null() {
            None
        } else {
            Some(Arc::new(FfiBootstrapClientObserver::new(observer_handle)))
        }
    }
}

impl Drop for FfiBootstrapClientObserverWeakPtr {
    fn drop(&mut self) {
        unsafe { DROP_WEAK.expect("DROP_WEAK missing")(self.handle) }
    }
}

pub type BootstrapClientClosedCallback = unsafe extern "C" fn(*mut c_void);

static mut CLIENT_CLOSED: Option<BootstrapClientClosedCallback> = None;

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_bootstrap_client_observer_closed(
    f: BootstrapClientClosedCallback,
) {
    CLIENT_CLOSED = Some(f);
}

/// takes a `shared_ptr<bootstrap_client_observer>*` and
pub type BootstrapClientObserverToWeakCallback = unsafe extern "C" fn(*mut c_void) -> *mut c_void;

static mut OBSERVER_TO_WEAK: Option<BootstrapClientObserverToWeakCallback> = None;
static mut WEAK_TO_OBSERVER: Option<BootstrapClientObserverToWeakCallback> = None;

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_bootstrap_client_observer_to_weak(
    f: BootstrapClientObserverToWeakCallback,
) {
    OBSERVER_TO_WEAK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_bootstrap_client_weak_to_observer(
    f: BootstrapClientObserverToWeakCallback,
) {
    WEAK_TO_OBSERVER = Some(f);
}

static mut DROP_WEAK: Option<VoidPointerCallback> = None;
static mut DROP_OBSERVER: Option<VoidPointerCallback> = None;

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_bootstrap_client_observer_destroy(f: VoidPointerCallback) {
    DROP_OBSERVER = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_bootstrap_client_observer_weak_destroy(
    f: VoidPointerCallback,
) {
    DROP_WEAK = Some(f);
}
