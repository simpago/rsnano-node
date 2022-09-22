use num::FromPrimitive;

use crate::{
    ffi::{DispatchCallback, ErrorCodeDto, LoggerHandle, StringDto, VoidPointerCallback},
    network::{Socket, SocketBuilder, SocketImpl, SocketType, TcpSocketFacade},
    stats::SocketStats,
    utils::{BufferHandle, BufferWrapper, ErrorCode},
};
use std::{
    ffi::c_void,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    ops::Deref,
    sync::{Arc, Mutex, Weak},
    time::Duration,
};

use crate::ffi::{
    thread_pool::{FfiThreadPool, VoidFnCallbackHandle},
    LoggerMT, StatHandle,
};

pub struct SocketHandle(Arc<SocketImpl>);
pub struct SocketWeakHandle(Weak<SocketImpl>);

impl SocketHandle {
    pub fn new(socket: Arc<SocketImpl>) -> *mut SocketHandle {
        Box::into_raw(Box::new(SocketHandle(socket)))
    }
}

impl Deref for SocketHandle {
    type Target = Arc<SocketImpl>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_create(
    endpoint_type: u8,
    tcp_facade: *mut c_void,
    stats_handle: *mut StatHandle,
    thread_pool: *mut c_void,
    default_timeout_s: u64,
    silent_connection_tolerance_time_s: u64,
    network_timeout_logging: bool,
    logger: *mut LoggerHandle,
) -> *mut SocketHandle {
    let endpoint_type = FromPrimitive::from_u8(endpoint_type).unwrap();
    let tcp_facade = Arc::new(FfiTcpSocketFacade::new(tcp_facade));
    let thread_pool = Arc::new(FfiThreadPool::new(thread_pool));
    let logger = Arc::new(LoggerMT::new(Box::from_raw(logger)));
    let stats = (*stats_handle).deref().clone();

    let socket_stats = Arc::new(SocketStats::new(stats, logger, network_timeout_logging));

    let socket = SocketBuilder::endpoint_type(endpoint_type, tcp_facade, thread_pool)
        .default_timeout(Duration::from_secs(default_timeout_s))
        .silent_connection_tolerance_time(Duration::from_secs(silent_connection_tolerance_time_s))
        .observer(socket_stats)
        .build();

    SocketHandle::new(socket)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_destroy(handle: *mut SocketHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_inner_ptr(handle: *mut SocketHandle) -> *const c_void {
    let p = Arc::as_ptr(&(*handle).0);
    p as *const c_void
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_to_weak_handle(
    handle: *mut SocketHandle,
) -> *mut SocketWeakHandle {
    Box::into_raw(Box::new(SocketWeakHandle(Arc::downgrade(&(*handle).0))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_weak_socket_destroy(handle: *mut SocketWeakHandle) {
    drop(Box::from_raw(handle));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_weak_socket_to_socket(
    handle: *mut SocketWeakHandle,
) -> *mut SocketHandle {
    match (*handle).0.upgrade() {
        Some(socket) => SocketHandle::new(socket),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_weak_socket_expired(handle: *mut SocketWeakHandle) -> bool {
    (*handle).0.strong_count() == 0
}

#[derive(Clone)]
#[repr(C)]
pub struct EndpointDto {
    pub bytes: [u8; 16],
    pub port: u16,
    pub v6: bool,
}

impl EndpointDto {
    pub fn new() -> EndpointDto {
        EndpointDto {
            bytes: [0; 16],
            port: 0,
            v6: false,
        }
    }
}

impl Default for EndpointDto {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&EndpointDto> for SocketAddr {
    fn from(dto: &EndpointDto) -> Self {
        let ip = if dto.v6 {
            IpAddr::V6(Ipv6Addr::from(dto.bytes))
        } else {
            let mut bytes = [0; 4];
            bytes.copy_from_slice(&dto.bytes[..4]);
            IpAddr::V4(Ipv4Addr::from(bytes))
        };

        SocketAddr::new(ip, dto.port)
    }
}

impl From<&SocketAddr> for EndpointDto {
    fn from(addr: &SocketAddr) -> Self {
        match addr {
            SocketAddr::V4(a) => {
                let mut dto = EndpointDto {
                    bytes: [0; 16],
                    port: a.port(),
                    v6: false,
                };
                dto.bytes[..4].copy_from_slice(&a.ip().octets());
                dto
            }
            SocketAddr::V6(a) => EndpointDto {
                bytes: a.ip().octets(),
                port: a.port(),
                v6: true,
            },
        }
    }
}

impl From<SocketAddr> for EndpointDto {
    fn from(addr: SocketAddr) -> Self {
        EndpointDto::from(&addr)
    }
}

type SocketConnectCallback = unsafe extern "C" fn(*mut c_void, *const ErrorCodeDto);
pub type SocketDestroyContext = unsafe extern "C" fn(*mut c_void);

struct ConnectCallbackWrapper {
    callback: SocketConnectCallback,
    destory_context: SocketDestroyContext,
    context: *mut c_void,
}

impl ConnectCallbackWrapper {
    fn new(
        callback: SocketConnectCallback,
        destory_context: SocketDestroyContext,
        context: *mut c_void,
    ) -> Self {
        Self {
            callback,
            destory_context,
            context,
        }
    }
    fn execute(&self, ec: ErrorCode) {
        let ec_dto = ErrorCodeDto::from(&ec);
        unsafe { (self.callback)(self.context, &ec_dto) };
    }
}

impl Drop for ConnectCallbackWrapper {
    fn drop(&mut self) {
        unsafe { (self.destory_context)(self.context) };
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_async_connect(
    handle: *mut SocketHandle,
    endpoint: *const EndpointDto,
    callback: SocketConnectCallback,
    destroy_context: SocketDestroyContext,
    context: *mut c_void,
) {
    let cb_wrapper = ConnectCallbackWrapper::new(callback, destroy_context, context);
    let cb = Box::new(move |ec| {
        cb_wrapper.execute(ec);
    });
    (*handle).async_connect((&*endpoint).into(), cb);
}

pub struct ReadCallbackWrapper {
    callback: SocketReadCallback,
    destory_context: SocketDestroyContext,
    context: *mut c_void,
}

impl ReadCallbackWrapper {
    pub fn new(
        callback: SocketReadCallback,
        destory_context: SocketDestroyContext,
        context: *mut c_void,
    ) -> Self {
        Self {
            callback,
            destory_context,
            context,
        }
    }

    pub fn execute(&self, ec: ErrorCode, size: usize) {
        let ec_dto = ErrorCodeDto::from(&ec);
        unsafe { (self.callback)(self.context, &ec_dto, size) };
    }
}

impl Drop for ReadCallbackWrapper {
    fn drop(&mut self) {
        unsafe { (self.destory_context)(self.context) };
    }
}

pub type SocketReadCallback = unsafe extern "C" fn(*mut c_void, *const ErrorCodeDto, usize);

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_async_read(
    handle: *mut SocketHandle,
    buffer: *mut c_void,
    size: usize,
    callback: SocketReadCallback,
    destroy_context: SocketDestroyContext,
    context: *mut c_void,
) {
    let cb_wrapper = ReadCallbackWrapper::new(callback, destroy_context, context);
    let cb = Box::new(move |ec, size| {
        cb_wrapper.execute(ec, size);
    });
    let buffer_wrapper = Arc::new(FfiBufferWrapper::new(buffer));
    (*handle).async_read(buffer_wrapper, size, cb);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_async_read2(
    handle: *mut SocketHandle,
    buffer: *mut BufferHandle,
    size: usize,
    callback: SocketReadCallback,
    destroy_context: SocketDestroyContext,
    context: *mut c_void,
) {
    let cb_wrapper = ReadCallbackWrapper::new(callback, destroy_context, context);
    let cb = Box::new(move |ec, size| {
        cb_wrapper.execute(ec, size);
    });
    (*handle).async_read2(Arc::clone(&(*buffer)), size, cb);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_async_write(
    handle: *mut SocketHandle,
    buffer: *const u8,
    buffer_len: usize,
    callback: SocketReadCallback,
    destroy_context: SocketDestroyContext,
    context: *mut c_void,
) {
    let cb: Option<Box<dyn FnOnce(ErrorCode, usize)>> = if !context.is_null() {
        let cb_wrapper = ReadCallbackWrapper::new(callback, destroy_context, context);
        Some(Box::new(move |ec, size| {
            cb_wrapper.execute(ec, size);
        }))
    } else {
        None
    };
    let buffer = std::slice::from_raw_parts(buffer, buffer_len);
    (*handle).async_write(&Arc::new(buffer.to_vec()), cb);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_local_endpoint(
    handle: *mut SocketHandle,
    endpoint: *mut EndpointDto,
) {
    let ep = (*handle).local_endpoint();
    set_enpoint_dto(&ep, &mut (*endpoint))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_set_remote_endpoint(
    handle: *mut SocketHandle,
    endpoint: *const EndpointDto,
) {
    (*handle).set_remote(SocketAddr::from(&*endpoint))
}

fn set_enpoint_dto(endpoint: &SocketAddr, result: &mut EndpointDto) {
    result.port = endpoint.port();
    match endpoint {
        SocketAddr::V4(addr) => {
            result.v6 = false;
            result.bytes[..4].copy_from_slice(&addr.ip().octets());
        }
        SocketAddr::V6(addr) => {
            result.v6 = true;
            result.bytes.copy_from_slice(&addr.ip().octets());
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_get_remote(
    handle: *mut SocketHandle,
    result: *mut EndpointDto,
) {
    match (*handle).get_remote() {
        Some(ep) => {
            set_enpoint_dto(&ep, &mut *result);
        }
        None => {
            (*result).port = 0;
            (*result).v6 = false;
            (*result).bytes = [0; 16];
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_endpoint_type(handle: *mut SocketHandle) -> u8 {
    (*handle).endpoint_type() as u8
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_max(handle: *mut SocketHandle) -> bool {
    (*handle).max()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_full(handle: *mut SocketHandle) -> bool {
    (*handle).full()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_set_silent_connection_tolerance_time(
    handle: *mut SocketHandle,
    time_s: u64,
) {
    (*handle).set_silent_connection_tolerance_time(time_s)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_set_timeout(handle: *mut SocketHandle, timeout_s: u64) {
    (*handle).set_timeout(Duration::from_secs(timeout_s));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_close(handle: *mut SocketHandle) {
    (*handle).close()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_close_internal(handle: *mut SocketHandle) {
    (*handle).close_internal();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_is_closed(handle: *mut SocketHandle) -> bool {
    (*handle).is_closed()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_has_timed_out(handle: *mut SocketHandle) -> bool {
    (*handle).has_timed_out()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_checkup(handle: *mut SocketHandle) {
    (*handle).checkup();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_get_queue_size(handle: *mut SocketHandle) -> usize {
    (*handle).get_queue_size()
}

pub struct AsyncConnectCallbackHandle(Option<Box<dyn FnOnce(ErrorCode)>>);

impl AsyncConnectCallbackHandle {
    pub fn new(callback: Box<dyn FnOnce(ErrorCode)>) -> Self {
        Self(Some(callback))
    }
}

type AsyncConnectCallback =
    unsafe extern "C" fn(*mut c_void, *const EndpointDto, *mut AsyncConnectCallbackHandle);

static mut ASYNC_CONNECT_CALLBACK: Option<AsyncConnectCallback> = None;

type RemoteEndpointCallback =
    unsafe extern "C" fn(*mut c_void, *mut EndpointDto, *mut ErrorCodeDto);

static mut REMOTE_ENDPOINT_CALLBACK: Option<RemoteEndpointCallback> = None;

static mut DISPATCH_CALLBACK: Option<DispatchCallback> = None;
static mut POST_CALLBACK: Option<DispatchCallback> = None;

type CloseSocketCallback = unsafe extern "C" fn(*mut c_void, *mut ErrorCodeDto);

static mut CLOSE_SOCKET_CALLBACK: Option<CloseSocketCallback> = None;

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_tcp_socket_async_connect(f: AsyncConnectCallback) {
    ASYNC_CONNECT_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_tcp_socket_remote_endpoint(f: RemoteEndpointCallback) {
    REMOTE_ENDPOINT_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_tcp_socket_dispatch(f: DispatchCallback) {
    DISPATCH_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_tcp_socket_post(f: DispatchCallback) {
    POST_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_tcp_socket_close(f: CloseSocketCallback) {
    CLOSE_SOCKET_CALLBACK = Some(f);
}

pub struct AsyncReadCallbackHandle(Option<Box<dyn FnOnce(ErrorCode, usize)>>);

type AsyncReadCallback =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *mut AsyncReadCallbackHandle);

type AsyncRead2Callback =
    unsafe extern "C" fn(*mut c_void, *mut BufferHandle, usize, *mut AsyncReadCallbackHandle);

static mut ASYNC_READ_CALLBACK: Option<AsyncReadCallback> = None;
static mut ASYNC_READ2_CALLBACK: Option<AsyncRead2Callback> = None;

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_tcp_socket_async_read(f: AsyncReadCallback) {
    ASYNC_READ_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_tcp_socket_async_read2(f: AsyncRead2Callback) {
    ASYNC_READ2_CALLBACK = Some(f);
}

pub struct AsyncWriteCallbackHandle(Option<Box<dyn FnOnce(ErrorCode, usize)>>);

impl AsyncWriteCallbackHandle {
    pub fn new(callback: Box<dyn FnOnce(ErrorCode, usize)>) -> Self {
        Self(Some(callback))
    }
}

type AsyncWriteCallback =
    unsafe extern "C" fn(*mut c_void, *const u8, usize, *mut AsyncWriteCallbackHandle);

static mut ASYNC_WRITE_CALLBACK: Option<AsyncWriteCallback> = None;

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_tcp_socket_async_write(f: AsyncWriteCallback) {
    ASYNC_WRITE_CALLBACK = Some(f);
}

static mut TCP_FACADE_DESTROY_CALLBACK: Option<VoidPointerCallback> = None;

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_tcp_socket_destroy(f: VoidPointerCallback) {
    TCP_FACADE_DESTROY_CALLBACK = Some(f);
}

type SocketLocalEndpointCallback = unsafe extern "C" fn(*mut c_void, *mut EndpointDto);

static mut LOCAL_ENDPOINT_CALLBACK: Option<SocketLocalEndpointCallback> = None;

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_tcp_socket_local_endpoint(f: SocketLocalEndpointCallback) {
    LOCAL_ENDPOINT_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_async_connect_callback_execute(
    callback: *mut AsyncConnectCallbackHandle,
    ec: *const ErrorCodeDto,
) {
    let error_code = ErrorCode::from(&*ec);
    if let Some(cb) = (*callback).0.take() {
        cb(error_code);
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_async_connect_callback_destroy(
    callback: *mut AsyncConnectCallbackHandle,
) {
    drop(Box::from_raw(callback))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_set_default_timeout_value(
    handle: *mut SocketHandle,
    timeout_s: u64,
) {
    (*handle).set_default_timeout_value(timeout_s)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_type(handle: *mut SocketHandle) -> u8 {
    (*handle).socket_type() as u8
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_set_type(handle: *mut SocketHandle, socket_type: u8) {
    (*handle).set_socket_type(SocketType::from_u8(socket_type).unwrap());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_facade(handle: *mut SocketHandle) -> *mut c_void {
    (*handle)
        .tcp_socket
        .as_any()
        .downcast_ref::<FfiTcpSocketFacade>()
        .expect("not an ffi socket")
        .handle
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_is_bootstrap_connection(handle: *mut SocketHandle) -> bool {
    (*handle).is_bootstrap_connection()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_default_timeout_value(handle: *mut SocketHandle) -> u64 {
    (*handle).default_timeout_value()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_socket_type_to_string(socket_type: u8, result: *mut StringDto) {
    *result = StringDto::from(SocketType::from_u8(socket_type).unwrap().as_str())
}

struct FfiTcpSocketFacade {
    handle: *mut c_void,
}

impl FfiTcpSocketFacade {
    fn new(handle: *mut c_void) -> Self {
        Self { handle }
    }
}

impl Drop for FfiTcpSocketFacade {
    fn drop(&mut self) {
        unsafe {
            TCP_FACADE_DESTROY_CALLBACK.expect("TCP_FACADE_DESTROY_CALLBACK missing")(self.handle)
        }
    }
}

impl TcpSocketFacade for FfiTcpSocketFacade {
    fn async_connect(&self, endpoint: SocketAddr, callback: Box<dyn FnOnce(ErrorCode)>) {
        let endpoint_dto = EndpointDto::from(&endpoint);
        let callback_handle = Box::new(AsyncConnectCallbackHandle::new(callback));
        unsafe {
            match ASYNC_CONNECT_CALLBACK {
                Some(f) => f(self.handle, &endpoint_dto, Box::into_raw(callback_handle)),
                None => panic!("ASYNC_CONNECT_CALLBACK missing"),
            }
        }
    }

    fn async_read(
        &self,
        buffer: &Arc<dyn BufferWrapper>,
        len: usize,
        callback: Box<dyn FnOnce(ErrorCode, usize)>,
    ) {
        let callback_handle = Box::into_raw(Box::new(AsyncReadCallbackHandle(Some(callback))));
        unsafe {
            ASYNC_READ_CALLBACK.expect("ASYNC_READ_CALLBACK missing")(
                self.handle,
                buffer.handle(),
                len,
                callback_handle,
            );
        }
    }

    fn async_read2(
        &self,
        buffer: &Arc<Mutex<Vec<u8>>>,
        len: usize,
        callback: Box<dyn FnOnce(ErrorCode, usize)>,
    ) {
        let callback_handle = Box::into_raw(Box::new(AsyncReadCallbackHandle(Some(callback))));
        unsafe {
            ASYNC_READ2_CALLBACK.expect("ASYNC_READ2_CALLBACK missing")(
                self.handle,
                BufferHandle::new(buffer.clone()),
                len,
                callback_handle,
            );
        }
    }

    fn async_write(&self, buffer: &Arc<Vec<u8>>, callback: Box<dyn FnOnce(ErrorCode, usize)>) {
        let callback_handle = Box::into_raw(Box::new(AsyncWriteCallbackHandle::new(callback)));
        unsafe {
            ASYNC_WRITE_CALLBACK.expect("ASYNC_WRITE_CALLBACK missing")(
                self.handle,
                buffer.as_ptr(),
                buffer.len(),
                callback_handle,
            );
        }
    }

    fn remote_endpoint(&self) -> Result<SocketAddr, ErrorCode> {
        let mut endpoint_dto = EndpointDto::new();
        let mut ec_dto = ErrorCodeDto {
            val: 0,
            category: 0,
        };
        unsafe {
            REMOTE_ENDPOINT_CALLBACK.expect("REMOTE_ENDPOINT_CALLBACK missing")(
                self.handle,
                &mut endpoint_dto,
                &mut ec_dto,
            );
        }
        if ec_dto.val == 0 {
            Ok((&endpoint_dto).into())
        } else {
            Err((&ec_dto).into())
        }
    }

    fn post(&self, f: Box<dyn FnOnce()>) {
        unsafe {
            POST_CALLBACK.expect("POST_CALLBACK missing")(
                self.handle,
                Box::into_raw(Box::new(VoidFnCallbackHandle::new(f))),
            );
        }
    }

    fn dispatch(&self, f: Box<dyn FnOnce()>) {
        unsafe {
            DISPATCH_CALLBACK.expect("DISPATCH_CALLBACK missing")(
                self.handle,
                Box::into_raw(Box::new(VoidFnCallbackHandle::new(f))),
            );
        }
    }

    fn close(&self) -> Result<(), ErrorCode> {
        let mut ec_dto = ErrorCodeDto {
            val: 0,
            category: 0,
        };
        unsafe {
            CLOSE_SOCKET_CALLBACK.expect("CLOSE_SOCKET_CALLBACK missing")(self.handle, &mut ec_dto);
        }

        if ec_dto.val == 0 {
            Ok(())
        } else {
            Err((&ec_dto).into())
        }
    }

    fn local_endpoint(&self) -> SocketAddr {
        unsafe {
            let mut dto = EndpointDto::new();
            LOCAL_ENDPOINT_CALLBACK.expect("LOCAL_ENDPOINT_CALLBACK missing")(
                self.handle,
                &mut dto,
            );
            SocketAddr::from(&dto)
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

unsafe impl Send for FfiTcpSocketFacade {}
unsafe impl Sync for FfiTcpSocketFacade {}

static mut BUFFER_DESTROY_CALLBACK: Option<VoidPointerCallback> = None;

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_buffer_destroy(f: VoidPointerCallback) {
    BUFFER_DESTROY_CALLBACK = Some(f);
}

type BufferSizeCallback = unsafe extern "C" fn(*mut c_void) -> usize;

static mut BUFFER_SIZE_CALLBACK: Option<BufferSizeCallback> = None;

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_buffer_size(f: BufferSizeCallback) {
    BUFFER_SIZE_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_async_read_callback_execute(
    callback: *mut AsyncReadCallbackHandle,
    ec: *const ErrorCodeDto,
    size: usize,
) {
    let error_code = ErrorCode::from(&*ec);
    (*callback).0.take().unwrap()(error_code, size);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_async_read_callback_destroy(callback: *mut AsyncReadCallbackHandle) {
    drop(Box::from_raw(callback))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_async_write_callback_execute(
    callback: *mut AsyncWriteCallbackHandle,
    ec: *const ErrorCodeDto,
    size: usize,
) {
    let error_code = ErrorCode::from(&*ec);
    if let Some(cb) = (*callback).0.take() {
        cb(error_code, size);
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_async_write_callback_destroy(callback: *mut AsyncWriteCallbackHandle) {
    drop(Box::from_raw(callback))
}

struct FfiBufferWrapper {
    handle: *mut c_void,
}

impl FfiBufferWrapper {
    fn new(handle: *mut c_void) -> Self {
        Self { handle }
    }
}

impl Drop for FfiBufferWrapper {
    fn drop(&mut self) {
        unsafe { BUFFER_DESTROY_CALLBACK.expect("BUFFER_DESTROY_CALLBACK missing")(self.handle) }
    }
}

impl BufferWrapper for FfiBufferWrapper {
    fn len(&self) -> usize {
        unsafe { BUFFER_SIZE_CALLBACK.expect("BUFFER_SIZE_CALLBACK missing")(self.handle) }
    }

    fn handle(&self) -> *mut c_void {
        self.handle
    }
}
