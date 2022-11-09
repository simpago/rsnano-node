use num_traits::FromPrimitive;
use std::{cmp::min, ffi::c_void, time::Duration};

use crate::{
    config::NetworkConstants,
    core::{Root, WorkVersion},
    ffi::{NetworkConstantsDto, VoidPointerCallback},
    utils::get_cpu_count,
    work::{WorkPool, WorkTicket},
};

pub struct WorkPoolHandle(WorkPool);

type OpenclCallback =
    unsafe extern "C" fn(*mut c_void, u8, *const u8, u64, *mut WorkTicketHandle, *mut u64) -> bool;

#[no_mangle]
pub unsafe extern "C" fn rsn_work_pool_create(
    network_constants: *const NetworkConstantsDto,
    max_threads: u32,
    pow_rate_limiter_ns: u64,
    opencl: OpenclCallback,
    opencl_context: *mut c_void,
    destroy_context: unsafe extern "C" fn(*mut c_void),
) -> *mut WorkPoolHandle {
    let network_constants = NetworkConstants::try_from(&*network_constants).unwrap();
    let thread_count = if network_constants.is_dev_network() {
        min(max_threads as usize, 1)
    } else {
        min(max_threads as usize, get_cpu_count())
    };
    Box::into_raw(Box::new(WorkPoolHandle(WorkPool::new(
        network_constants.work,
        thread_count,
        Duration::from_nanos(pow_rate_limiter_ns),
        create_opencl_wrapper(opencl, opencl_context, destroy_context),
    ))))
}

struct OpenclWrapper {
    callback: OpenclCallback,
    destroy_context: unsafe extern "C" fn(*mut c_void),
    context: *mut c_void,
}

impl OpenclWrapper {
    fn callback(
        &self,
        version: WorkVersion,
        root: Root,
        difficulty: u64,
        ticket: &WorkTicket,
    ) -> Option<u64> {
        let mut work = 0;
        let ticket =
            unsafe { std::mem::transmute::<WorkTicket, WorkTicket<'static>>(ticket.clone()) };
        let ticket_handle = Box::into_raw(Box::new(WorkTicketHandle(ticket)));
        let found = unsafe {
            (self.callback)(
                self.context,
                version as u8,
                root.as_bytes().as_ptr(),
                difficulty,
                ticket_handle,
                &mut work,
            )
        };
        if found {
            Some(work)
        } else {
            None
        }
    }
}

impl Drop for OpenclWrapper {
    fn drop(&mut self) {
        unsafe {
            (self.destroy_context)(self.context);
        }
    }
}

unsafe impl Send for OpenclWrapper {}
unsafe impl Sync for OpenclWrapper {}

fn create_opencl_wrapper(
    opencl: OpenclCallback,
    context: *mut c_void,
    destroy_context: unsafe extern "C" fn(*mut c_void),
) -> Option<Box<dyn Fn(WorkVersion, Root, u64, &WorkTicket) -> Option<u64> + Send + Sync>> {
    if context.is_null() {
        return None;
    }

    let wrapper = OpenclWrapper {
        callback: opencl,
        destroy_context,
        context,
    };

    Some(Box::new(move |version, root, difficulty, ticket| {
        wrapper.callback(version, root, difficulty, ticket)
    }))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_pool_destroy(handle: *mut WorkPoolHandle) {
    drop(Box::from_raw(handle));
}

pub struct WorkTicketHandle(WorkTicket<'static>);

#[no_mangle]
pub unsafe extern "C" fn rsn_work_ticket_create() -> *mut WorkTicketHandle {
    Box::into_raw(Box::new(WorkTicketHandle(WorkTicket::never_expires())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_ticket_clone(
    handle: *mut WorkTicketHandle,
) -> *mut WorkTicketHandle {
    Box::into_raw(Box::new(WorkTicketHandle((*handle).0.clone())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_ticket_destroy(handle: *mut WorkTicketHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_ticket_expired(handle: *mut WorkTicketHandle) -> bool {
    (*handle).0.expired()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_pool_has_opencl(handle: *mut WorkPoolHandle) -> bool {
    (*handle).0.has_opencl()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_pool_cancel(handle: *mut WorkPoolHandle, root: *const u8) {
    (*handle).0.cancel(&Root::from_ptr(root));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_pool_stop(handle: *mut WorkPoolHandle) {
    (*handle).0.stop();
}

pub type WorkPoolDoneCallback = unsafe extern "C" fn(*mut c_void, u64, bool);

struct WorkPoolDoneWrapper {
    callback: WorkPoolDoneCallback,
    context: *mut c_void,
    destroy: VoidPointerCallback,
}

impl WorkPoolDoneWrapper {
    pub fn done(&self, work: Option<u64>) {
        unsafe { (self.callback)(self.context, work.unwrap_or_default(), work.is_some()) };
    }
}

impl Drop for WorkPoolDoneWrapper {
    fn drop(&mut self) {
        unsafe { (self.destroy)(self.context) }
    }
}

unsafe impl Send for WorkPoolDoneWrapper {}
unsafe impl Sync for WorkPoolDoneWrapper {}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_pool_generate_async(
    handle: *mut WorkPoolHandle,
    version: u8,
    root: *const u8,
    difficulty: u64,
    done: WorkPoolDoneCallback,
    context: *mut c_void,
    destroy_context: VoidPointerCallback,
) {
    let done_callback: Option<Box<dyn Fn(Option<u64>) + Send>> = if context.is_null() {
        None
    } else {
        let wrapper = WorkPoolDoneWrapper {
            callback: done,
            context,
            destroy: destroy_context,
        };
        Some(Box::new(move |work| wrapper.done(work)))
    };
    (*handle).0.generate_async(
        WorkVersion::from_u8(version).unwrap(),
        Root::from_ptr(root),
        difficulty,
        done_callback,
    );
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_pool_generate_dev(
    handle: *mut WorkPoolHandle,
    root: *const u8,
    difficulty: u64,
    result: *mut u64,
) -> bool {
    match (*handle).0.generate_dev(Root::from_ptr(root), difficulty) {
        Some(work) => {
            unsafe { *result = work };
            true
        }
        None => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_pool_generate_dev2(
    handle: *mut WorkPoolHandle,
    root: *const u8,
    result: *mut u64,
) -> bool {
    match (*handle).0.generate_dev2(Root::from_ptr(root)) {
        Some(work) => {
            unsafe { *result = work };
            true
        }
        None => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_pool_generate(
    handle: *mut WorkPoolHandle,
    version: u8,
    root: *const u8,
    difficulty: u64,
    result: *mut u64,
) -> bool {
    match (*handle).0.generate(
        WorkVersion::from_u8(version).unwrap(),
        Root::from_ptr(root),
        difficulty,
    ) {
        Some(work) => {
            unsafe { *result = work };
            true
        }
        None => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_pool_size(handle: *mut WorkPoolHandle) -> usize {
    (*handle).0.size()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_pool_pending_value_size() -> usize {
    WorkPool::pending_value_size()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_pool_thread_count(handle: *mut WorkPoolHandle) -> usize {
    (*handle).0.thread_count()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_pool_threshold_base(
    handle: *mut WorkPoolHandle,
    version: u8,
) -> u64 {
    (*handle)
        .0
        .threshold_base(WorkVersion::from_u8(version).unwrap())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_pool_difficulty(
    handle: *mut WorkPoolHandle,
    version: u8,
    root: *const u8,
    work: u64,
) -> u64 {
    (*handle).0.difficulty(
        WorkVersion::from_u8(version).unwrap(),
        &Root::from_ptr(root),
        work,
    )
}
