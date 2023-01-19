use std::sync::Arc;

use num_traits::FromPrimitive;
use rsnano_ledger::{WriteDatabaseQueue, WriteGuard, Writer};

pub struct WriteDatabaseQueueHandle(pub Arc<WriteDatabaseQueue>);

#[no_mangle]
pub extern "C" fn rsn_write_database_queue_create(use_noop: bool) -> *mut WriteDatabaseQueueHandle {
    Box::into_raw(Box::new(WriteDatabaseQueueHandle(Arc::new(
        WriteDatabaseQueue::new(use_noop),
    ))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_write_database_queue_destroy(handle: *mut WriteDatabaseQueueHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_write_database_queue_wait(
    handle: *mut WriteDatabaseQueueHandle,
    writer: u8,
) -> *mut WriteGuardHandle {
    let guard = (*handle).0.wait(Writer::from_u8(writer).unwrap());
    WriteGuardHandle::new(guard)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_write_database_queue_contains(
    handle: *mut WriteDatabaseQueueHandle,
    writer: u8,
) -> bool {
    (*handle).0.contains(Writer::from_u8(writer).unwrap())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_write_database_queue_process(
    handle: *mut WriteDatabaseQueueHandle,
    writer: u8,
) -> bool {
    (*handle).0.process(Writer::from_u8(writer).unwrap())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_write_database_queue_pop(
    handle: *mut WriteDatabaseQueueHandle,
) -> *mut WriteGuardHandle {
    WriteGuardHandle::new((*handle).0.pop())
}

pub struct WriteGuardHandle(pub WriteGuard);
impl WriteGuardHandle {
    pub fn new(guard: WriteGuard) -> *mut WriteGuardHandle {
        Box::into_raw(Box::new(WriteGuardHandle(guard)))
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_write_guard_release(handle: *mut WriteGuardHandle) {
    (*handle).0.release();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_write_guard_destroy(handle: *mut WriteGuardHandle) {
    drop(Box::from_raw(handle))
}
