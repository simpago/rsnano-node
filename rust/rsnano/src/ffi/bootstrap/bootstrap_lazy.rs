use std::{
    ffi::{c_void, CStr},
    os::raw::c_char,
    sync::Arc,
};

use crate::{
    bootstrap::{BootstrapAttemptLazy, BootstrapStrategy},
    ffi::{
        block_processing::BlockProcessorHandle,
        ledger::datastore::LedgerHandle,
        utils::{LoggerHandle, LoggerMT},
        FfiListener,
    },
    websocket::{Listener, NullListener},
};

use super::{
    bootstrap_attempt::BootstrapAttemptHandle, bootstrap_initiator::BootstrapInitiatorHandle,
};

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_attempt_lazy_create(
    logger: *mut LoggerHandle,
    websocket_server: *mut c_void,
    block_processor: *const BlockProcessorHandle,
    bootstrap_initiator: *const BootstrapInitiatorHandle,
    ledger: *const LedgerHandle,
    id: *const c_char,
    incremental_id: u64,
) -> *mut BootstrapAttemptHandle {
    let logger = Arc::new(LoggerMT::new(Box::from_raw(logger)));
    let id_str = CStr::from_ptr(id).to_str().unwrap();
    let websocket_server: Arc<dyn Listener> = if websocket_server.is_null() {
        Arc::new(NullListener::new())
    } else {
        Arc::new(FfiListener::new(websocket_server))
    };
    let block_processor = Arc::downgrade(&*block_processor);
    let bootstrap_initiator = Arc::downgrade(&*bootstrap_initiator);
    let ledger = Arc::clone(&*ledger);
    BootstrapAttemptHandle::new(Arc::new(BootstrapStrategy::Lazy(
        BootstrapAttemptLazy::new(
            logger,
            websocket_server,
            block_processor,
            bootstrap_initiator,
            ledger,
            id_str,
            incremental_id,
        )
        .unwrap(),
    )))
}
