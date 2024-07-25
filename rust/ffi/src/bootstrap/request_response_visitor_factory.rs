use super::bootstrap_initiator::BootstrapInitiatorHandle;
use crate::{
    block_processing::BlockProcessorHandle,
    ledger::datastore::LedgerHandle,
    transport::SynCookiesHandle,
    utils::{AsyncRuntimeHandle, ThreadPoolHandle},
    NetworkParamsDto, NodeConfigDto, NodeFlagsHandle, StatHandle,
};
use rsnano_core::KeyPair;
use rsnano_node::{bootstrap::BootstrapMessageVisitorFactory, NetworkParams};
use std::sync::Arc;

pub struct RequestResponseVisitorFactoryHandle(pub Arc<BootstrapMessageVisitorFactory>);

#[repr(C)]
pub struct RequestResponseVisitorFactoryParams {
    pub async_rt: *mut AsyncRuntimeHandle,
    pub config: *const NodeConfigDto,
    pub workers: *mut ThreadPoolHandle,
    pub network: *const NetworkParamsDto,
    pub stats: *mut StatHandle,
    pub syn_cookies: *mut SynCookiesHandle,
    pub node_id_prv: *const u8,
    pub ledger: *mut LedgerHandle,
    pub block_processor: *mut BlockProcessorHandle,
    pub bootstrap_initiator: *mut BootstrapInitiatorHandle,
    pub flags: *const NodeFlagsHandle,
}

#[no_mangle]
pub unsafe extern "C" fn rsn_request_response_visitor_factory_create(
    params: &RequestResponseVisitorFactoryParams,
) -> *mut RequestResponseVisitorFactoryHandle {
    let async_rt = Arc::clone(&(*params.async_rt).0);
    let workers = (*params.workers).0.clone();
    let network = NetworkParams::try_from(&*params.network).unwrap();
    let stats = Arc::clone(&(*params.stats));
    let ledger = Arc::clone(&(*params.ledger));
    let block_processor = Arc::clone(&(*params.block_processor));
    let bootstrap_initiator = Arc::clone(&(*params.bootstrap_initiator));
    let node_flags = (*params.flags).0.lock().unwrap().clone();
    let node_id =
        KeyPair::from_priv_key_bytes(std::slice::from_raw_parts(params.node_id_prv, 32)).unwrap();

    let visitor_factory = BootstrapMessageVisitorFactory::new(
        async_rt,
        Arc::clone(&stats),
        network.network.clone(),
        ledger,
        workers,
        block_processor,
        bootstrap_initiator,
        node_flags,
    );
    Box::into_raw(Box::new(RequestResponseVisitorFactoryHandle(Arc::new(
        visitor_factory,
    ))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_request_response_visitor_factory_destroy(
    handle: *mut RequestResponseVisitorFactoryHandle,
) {
    drop(Box::from_raw(handle))
}
