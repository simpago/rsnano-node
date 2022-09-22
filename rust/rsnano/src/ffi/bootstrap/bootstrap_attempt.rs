use std::{
    ffi::{c_void, CStr, CString},
    ops::Deref,
    os::raw::c_char,
    sync::{atomic::Ordering, Arc, MutexGuard},
    time::Duration,
};

use num::FromPrimitive;

use crate::{
    bootstrap::BootstrapStrategy,
    ffi::{datastore::LedgerHandle, LoggerHandle},
    websocket::{Listener, NullListener},
    Account, BootstrapAttempt,
};

use crate::ffi::{
    BlockHandle, BlockProcessorHandle, FfiListener, LoggerMT, StringDto, StringHandle,
};

use super::bootstrap_initiator::BootstrapInitiatorHandle;

pub struct BootstrapAttemptHandle(Arc<BootstrapStrategy>);

impl BootstrapAttemptHandle {
    pub fn new(strategy: Arc<BootstrapStrategy>) -> *mut BootstrapAttemptHandle {
        Box::into_raw(Box::new(BootstrapAttemptHandle(strategy)))
    }
}

impl Deref for BootstrapAttemptHandle {
    type Target = Arc<BootstrapStrategy>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_create(
    logger: *mut LoggerHandle,
    websocket_server: *mut c_void,
    block_processor: *const BlockProcessorHandle,
    bootstrap_initiator: *const BootstrapInitiatorHandle,
    ledger: *const LedgerHandle,
    id: *const c_char,
    mode: u8,
    incremental_id: u64,
) -> *mut BootstrapAttemptHandle {
    let logger = Arc::new(LoggerMT::new(Box::from_raw(logger)));
    let id_str = CStr::from_ptr(id).to_str().unwrap();
    let mode = FromPrimitive::from_u8(mode).unwrap();
    let websocket_server: Arc<dyn Listener> = if websocket_server.is_null() {
        Arc::new(NullListener::new())
    } else {
        Arc::new(FfiListener::new(websocket_server))
    };
    let block_processor = Arc::downgrade(&*block_processor);
    let bootstrap_initiator = Arc::downgrade(&*bootstrap_initiator);
    let ledger = Arc::clone(&*ledger);
    BootstrapAttemptHandle::new(Arc::new(BootstrapStrategy::Other(
        BootstrapAttempt::new(
            logger,
            websocket_server,
            block_processor,
            bootstrap_initiator,
            ledger,
            id_str,
            mode,
            incremental_id,
        )
        .unwrap(),
    )))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_destroy(handle: *mut BootstrapAttemptHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_stop(handle: *mut BootstrapAttemptHandle) {
    (*handle).0.attempt().stop();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_id(
    handle: *const BootstrapAttemptHandle,
    result: *mut StringDto,
) {
    let id = CString::new((*handle).0.attempt().id.as_str()).unwrap();
    let string_handle = Box::new(StringHandle(id));
    let result = &mut (*result);
    result.value = string_handle.0.as_ptr();
    result.handle = Box::into_raw(string_handle);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_should_log(
    handle: *const BootstrapAttemptHandle,
) -> bool {
    (*handle).0.attempt().should_log()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_bootstrap_mode(
    handle: *const BootstrapAttemptHandle,
) -> u8 {
    (*handle).0.attempt().mode as u8
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_bootstrap_mode_text(
    handle: *const BootstrapAttemptHandle,
    len: *mut usize,
) -> *const c_char {
    let mode_text = (*handle).0.attempt().mode_text();
    *len = mode_text.len();
    mode_text.as_ptr() as *const c_char
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_total_blocks(
    handle: *const BootstrapAttemptHandle,
) -> u64 {
    (*handle).0.attempt().total_blocks.load(Ordering::SeqCst)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_total_blocks_inc(
    handle: *const BootstrapAttemptHandle,
) {
    (*handle)
        .0
        .attempt()
        .total_blocks
        .fetch_add(1, Ordering::SeqCst);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_process_block(
    handle: *const BootstrapAttemptHandle,
    block: *const BlockHandle,
    known_account: *const u8,
    pull_blocks_processed: u64,
    max_blocks: u32,
    block_expected: bool,
    retry_limit: u32,
) -> bool {
    let block = (*block).block.clone();
    (*handle).0.attempt().process_block(
        block,
        &Account::from_ptr(known_account),
        pull_blocks_processed,
        max_blocks,
        block_expected,
        retry_limit,
    )
}

pub struct BootstrapAttemptLockHandle(Option<MutexGuard<'static, u8>>);

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_lock(
    handle: *mut BootstrapAttemptHandle,
) -> *mut BootstrapAttemptLockHandle {
    let guard = (*handle).0.attempt().mutex.lock().unwrap();
    Box::into_raw(Box::new(BootstrapAttemptLockHandle(Some(
        std::mem::transmute::<MutexGuard<u8>, MutexGuard<'static, u8>>(guard),
    ))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_unlock(handle: *mut BootstrapAttemptLockHandle) {
    drop(Box::from_raw(handle));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_notifiy_all(handle: *mut BootstrapAttemptHandle) {
    (*handle).0.attempt().condition.notify_all();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_wait(
    handle: *mut BootstrapAttemptHandle,
    lck: *mut BootstrapAttemptLockHandle,
) {
    let guard = (*handle)
        .0
        .attempt()
        .condition
        .wait((*lck).0.take().unwrap())
        .unwrap();
    (*lck).0 = Some(std::mem::transmute::<MutexGuard<u8>, MutexGuard<'static, u8>>(guard));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_wait_for(
    handle: *mut BootstrapAttemptHandle,
    lck: *mut BootstrapAttemptLockHandle,
    timeout_millis: u64,
) {
    let (guard, _) = (*handle)
        .0
        .attempt()
        .condition
        .wait_timeout(
            (*lck).0.take().unwrap(),
            Duration::from_millis(timeout_millis),
        )
        .unwrap();
    (*lck).0 = Some(std::mem::transmute::<MutexGuard<u8>, MutexGuard<'static, u8>>(guard));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_incremental_id(
    handle: *const BootstrapAttemptHandle,
) -> u64 {
    (*handle).0.attempt().incremental_id
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_pulling(
    handle: *const BootstrapAttemptHandle,
) -> u32 {
    (*handle).0.attempt().pulling.load(Ordering::SeqCst)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_pulling_inc(handle: *mut BootstrapAttemptHandle) {
    (*handle).0.attempt().pulling.fetch_add(1, Ordering::SeqCst);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_pull_started(handle: *mut BootstrapAttemptHandle) {
    (*handle).0.attempt().pull_started();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_pull_finished(handle: *mut BootstrapAttemptHandle) {
    (*handle).0.attempt().pull_finished();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_started(
    handle: *const BootstrapAttemptHandle,
) -> bool {
    (*handle).0.attempt().started.load(Ordering::SeqCst)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_set_started(
    handle: *mut BootstrapAttemptHandle,
) -> bool {
    (*handle).0.attempt().started.swap(true, Ordering::SeqCst)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_stopped(
    handle: *const BootstrapAttemptHandle,
) -> bool {
    (*handle).0.attempt().stopped()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_set_stopped(handle: *mut BootstrapAttemptHandle) {
    (*handle).0.attempt().set_stopped()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_still_pulling(
    handle: *const BootstrapAttemptHandle,
) -> bool {
    (*handle).0.attempt().still_pulling()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_requeued_pulls(
    handle: *const BootstrapAttemptHandle,
) -> u32 {
    (*handle).0.attempt().requeued_pulls.load(Ordering::SeqCst)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_requeued_pulls_inc(
    handle: *const BootstrapAttemptHandle,
) {
    (*handle)
        .0
        .attempt()
        .requeued_pulls
        .fetch_add(1, Ordering::SeqCst);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_frontiers_received(
    handle: *const BootstrapAttemptHandle,
) -> bool {
    (*handle)
        .0
        .attempt()
        .frontiers_received
        .load(Ordering::SeqCst)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_frontiers_received_set(
    handle: *mut BootstrapAttemptHandle,
    received: bool,
) {
    (*handle)
        .0
        .attempt()
        .frontiers_received
        .store(received, Ordering::SeqCst)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_duration_seconds(
    handle: *const BootstrapAttemptHandle,
) -> u64 {
    (*handle).0.attempt().duration().as_secs()
}
