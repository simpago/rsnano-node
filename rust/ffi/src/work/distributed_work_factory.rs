use crate::{core::BlockHandle, utils::ContextWrapper, VoidPointerCallback};
use rsnano_core::{Account, Root, WorkVersion};
use rsnano_node::work::DistributedWorkFactory;
use std::{ffi::c_void, ops::Deref, sync::Arc};
use tokio::task::spawn_blocking;

pub struct DistributedWorkFactoryHandle(pub Arc<DistributedWorkFactory>);

impl Deref for DistributedWorkFactoryHandle {
    type Target = Arc<DistributedWorkFactory>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_distributed_work_factory_destroy(
    handle: *mut DistributedWorkFactoryHandle,
) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub extern "C" fn rsn_distributed_work_factory_enabled(
    handle: &DistributedWorkFactoryHandle,
) -> bool {
    handle.work_generation_enabled()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_distributed_work_factory_cancel(
    handle: &DistributedWorkFactoryHandle,
    root: *const u8,
) {
    handle.cancel(Root::from_ptr(root))
}

pub type WorkResultCallback = unsafe extern "C" fn(*mut c_void, bool, u64);

#[no_mangle]
pub unsafe extern "C" fn rsn_distributed_work_factory_make(
    handle: &DistributedWorkFactoryHandle,
    root: *const u8,
    difficulty: u64,
    account: *const u8,
    callback: WorkResultCallback,
    context: *mut c_void,
    delete_context: VoidPointerCallback,
) {
    let account = if account.is_null() {
        None
    } else {
        Some(Account::from_ptr(account))
    };
    let factory = Arc::clone(handle);
    let context = ContextWrapper::new(context, delete_context);
    let root = Root::from_ptr(root);

    handle.tokio.spawn(async move {
        let work = factory.make(root, difficulty, account).await;
        spawn_blocking(move || {
            callback(
                context.get_context(),
                work.is_some(),
                work.unwrap_or_default(),
            );
        })
    });
}

#[no_mangle]
pub unsafe extern "C" fn rsn_distributed_work_factory_make_blocking_block(
    handle: &DistributedWorkFactoryHandle,
    block: &mut BlockHandle,
    difficulty: u64,
    work: *mut u64,
) -> bool {
    let block = block.get_mut();
    if let Some(result) = handle.make_blocking_block(block, difficulty) {
        *work = result;
        true
    } else {
        false
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_distributed_work_factory_make_blocking(
    handle: &DistributedWorkFactoryHandle,
    root: *const u8,
    difficulty: u64,
    account: *const u8,
    work: *mut u64,
) -> bool {
    let account = if account.is_null() {
        None
    } else {
        Some(Account::from_ptr(account))
    };
    if let Some(result) = handle.make_blocking(
        WorkVersion::Work1,
        Root::from_ptr(root),
        difficulty,
        account,
    ) {
        *work = result;
        true
    } else {
        false
    }
}
