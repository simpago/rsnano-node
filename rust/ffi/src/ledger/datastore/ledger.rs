use super::{
    ledger_set_any::LedgerSetAnyHandle,
    ledger_set_confirmed::LedgerSetConfirmedHandle,
    lmdb::{PendingInfoDto, PendingKeyDto, TransactionHandle},
    write_queue::WriteGuardHandle,
};
use crate::{
    core::{copy_block_array_dto, AccountInfoHandle, BlockArrayDto, BlockHandle},
    StringDto,
};
use num_traits::FromPrimitive;
use rsnano_core::{Account, BlockEnum, BlockHash, Epoch, Link};
use rsnano_ledger::{
    AnyReceivableIterator, BlockStatus, Ledger, LedgerSetAny, LedgerSetConfirmed, Writer,
};
use std::{ops::Deref, ptr::null_mut, sync::Arc};

pub struct LedgerHandle(pub Arc<Ledger>);

impl Deref for LedgerHandle {
    type Target = Arc<Ledger>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub extern "C" fn rsn_ledger_destroy(handle: *mut LedgerHandle) {
    drop(unsafe { Box::from_raw(handle) });
}

#[no_mangle]
pub extern "C" fn rsn_ledger_wait(handle: &LedgerHandle) -> *mut WriteGuardHandle {
    WriteGuardHandle::new(handle.write_queue.wait(Writer::Testing))
}

#[no_mangle]
pub extern "C" fn rsn_ledger_queue_contains(handle: &LedgerHandle, writer: u8) -> bool {
    handle
        .write_queue
        .contains(Writer::from_u8(writer).unwrap())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_pruning_enabled(handle: *mut LedgerHandle) -> bool {
    (*handle).0.pruning_enabled()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_enable_pruning(handle: *mut LedgerHandle) {
    (*handle).0.enable_pruning()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_any(handle: &LedgerHandle) -> *mut LedgerSetAnyHandle {
    let any = std::mem::transmute::<LedgerSetAny, LedgerSetAny<'static>>(handle.any());
    Box::into_raw(Box::new(LedgerSetAnyHandle(any)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_confirmed(
    handle: &LedgerHandle,
) -> *mut LedgerSetConfirmedHandle {
    let any =
        std::mem::transmute::<LedgerSetConfirmed, LedgerSetConfirmed<'static>>(handle.confirmed());
    Box::into_raw(Box::new(LedgerSetConfirmedHandle(any)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_account_receivable(
    handle: *mut LedgerHandle,
    txn: *mut TransactionHandle,
    account: *const u8,
    only_confirmed: bool,
    result: *mut u8,
) {
    let balance = (*handle).0.account_receivable(
        (*txn).as_txn(),
        &Account::from_ptr(account),
        only_confirmed,
    );
    balance.copy_bytes(result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_block_text(
    handle: *mut LedgerHandle,
    hash: *const u8,
    result: *mut StringDto,
) {
    *result = match (*handle).0.block_text(&BlockHash::from_ptr(hash)) {
        Ok(s) => s.into(),
        Err(_) => "".into(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_hash_root_random(
    handle: *mut LedgerHandle,
    txn: *mut TransactionHandle,
    result_hash: *mut u8,
    result_root: *mut u8,
) {
    let (hash, root) = (*handle)
        .0
        .hash_root_random((*txn).as_txn())
        .unwrap_or_default();
    hash.copy_bytes(result_hash);
    root.copy_bytes(result_root);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_weight(
    handle: *mut LedgerHandle,
    account: *const u8,
    result: *mut u8,
) {
    let weight = (*handle).0.weight(&Account::from_ptr(account));
    weight.copy_bytes(result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_weight_exact(
    handle: *mut LedgerHandle,
    txn: &TransactionHandle,
    account: *const u8,
    result: *mut u8,
) {
    let weight = (*handle)
        .0
        .weight_exact(txn.as_txn(), Account::from_ptr(account));
    weight.copy_bytes(result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_version(
    handle: &LedgerHandle,
    txn: &mut TransactionHandle,
    hash: *const u8,
) -> u8 {
    handle.0.version(txn.as_txn(), &BlockHash::from_ptr(hash)) as u8
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_latest_root(
    handle: *mut LedgerHandle,
    txn: *mut TransactionHandle,
    account: *const u8,
    result: *mut u8,
) {
    let latest = (*handle)
        .0
        .latest_root((*txn).as_txn(), &Account::from_ptr(account));
    latest.copy_bytes(result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_is_epoch_link(
    handle: *mut LedgerHandle,
    link: *const u8,
) -> bool {
    (*handle).0.is_epoch_link(&Link::from_ptr(link))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_find_receive_block_by_send_hash(
    handle: *mut LedgerHandle,
    txn: *mut TransactionHandle,
    destination: *const u8,
    send_block_hash: *const u8,
) -> *mut BlockHandle {
    let block = (*handle).0.find_receive_block_by_send_hash(
        (*txn).as_txn(),
        &Account::from_ptr(destination),
        &BlockHash::from_ptr(send_block_hash),
    );
    match block {
        Some(b) => Box::into_raw(Box::new(BlockHandle(Arc::new(b)))),
        None => null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_epoch_signer(
    handle: *mut LedgerHandle,
    link: *const u8,
    result: *mut u8,
) {
    let signer = (*handle)
        .0
        .constants
        .epochs
        .epoch_signer(&Link::from_ptr(link))
        .unwrap_or_default();
    signer.copy_bytes(result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_epoch_link(
    handle: *mut LedgerHandle,
    epoch: u8,
    result: *mut u8,
) {
    let link = (*handle)
        .0
        .epoch_link(Epoch::from_u8(epoch).unwrap())
        .unwrap_or_default();
    link.copy_bytes(result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_update_account(
    handle: *mut LedgerHandle,
    txn: *mut TransactionHandle,
    account: *const u8,
    old_info: *const AccountInfoHandle,
    new_info: *const AccountInfoHandle,
) {
    (*handle).0.update_account(
        (*txn).as_write_txn(),
        &Account::from_ptr(account),
        &*old_info,
        &*new_info,
    );
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_pruning_action(
    handle: *mut LedgerHandle,
    txn: *mut TransactionHandle,
    hash: *const u8,
    batch_size: u64,
) -> u64 {
    (*handle).0.pruning_action(
        (*txn).as_write_txn(),
        &BlockHash::from_ptr(hash),
        batch_size,
    )
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_representative(
    handle: &LedgerHandle,
    txn: &TransactionHandle,
    hash: *const u8,
    result: *mut u8,
) {
    let representative = handle
        .0
        .representative_block_hash(txn.as_txn(), &BlockHash::from_ptr(hash));
    representative.copy_bytes(result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_rollback(
    handle: *mut LedgerHandle,
    txn: *mut TransactionHandle,
    hash: *const u8,
    result: &mut BlockArrayDto,
) -> bool {
    match (*handle)
        .0
        .rollback((*txn).as_write_txn(), &BlockHash::from_ptr(hash))
    {
        Ok(mut block_list) => {
            let block_list = block_list
                .drain(..)
                .map(|b| Arc::new(b))
                .collect::<Vec<_>>();
            copy_block_array_dto(block_list, result);
            false
        }
        Err(_) => {
            copy_block_array_dto(Vec::new(), result);
            true
        }
    }
}

#[repr(C)]
pub struct ProcessReturnDto {
    pub code: u8,
}

impl From<BlockStatus> for ProcessReturnDto {
    fn from(result: BlockStatus) -> Self {
        Self { code: result as u8 }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_process(
    handle: &LedgerHandle,
    txn: &mut TransactionHandle,
    block: &mut BlockHandle,
    result: *mut ProcessReturnDto,
) {
    // this is undefined behaviour and should be fixed ASAP:
    let block_ptr = Arc::as_ptr(&block) as *mut BlockEnum;
    let res = handle.0.process(txn.as_write_txn(), &mut *block_ptr);
    let res = match res {
        Ok(()) => BlockStatus::Progress,
        Err(res) => res,
    };
    (*result) = res.into();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_confirm(
    handle: &mut LedgerHandle,
    txn: &mut TransactionHandle,
    hash: *const u8,
    result: &mut BlockArrayDto,
) {
    let hash = BlockHash::from_ptr(hash);
    let confirmed: Vec<_> = handle
        .confirm(txn.as_write_txn(), hash)
        .drain(..)
        .map(Arc::new)
        .collect();
    copy_block_array_dto(confirmed, result);
}

pub struct ReceivableIteratorHandle(AnyReceivableIterator<'static>);

impl ReceivableIteratorHandle {
    pub unsafe fn new<'a>(it: AnyReceivableIterator<'a>) -> *mut Self {
        let it =
            std::mem::transmute::<AnyReceivableIterator<'a>, AnyReceivableIterator<'static>>(it);
        Box::into_raw(Box::new(ReceivableIteratorHandle(it)))
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_receivable_iterator_destroy(handle: *mut ReceivableIteratorHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub extern "C" fn rsn_receivable_iterator_next(
    handle: &mut ReceivableIteratorHandle,
    key: &mut PendingKeyDto,
    info: &mut PendingInfoDto,
) -> bool {
    match handle.0.next() {
        Some((k, i)) => {
            *key = k.into();
            *info = i.into();
            true
        }
        None => false,
    }
}

#[no_mangle]
pub extern "C" fn rsn_ledger_cemented_count(handle: &LedgerHandle) -> u64 {
    handle.cemented_count()
}

#[no_mangle]
pub extern "C" fn rsn_ledger_block_count(handle: &LedgerHandle) -> u64 {
    handle.block_count()
}

#[no_mangle]
pub extern "C" fn rsn_ledger_account_count(handle: &LedgerHandle) -> u64 {
    handle.account_count()
}

#[no_mangle]
pub extern "C" fn rsn_ledger_pruned_count(handle: &LedgerHandle) -> u64 {
    handle.pruned_count()
}

#[no_mangle]
pub extern "C" fn rsn_ledger_dependents_confirmed(
    handle: &LedgerHandle,
    txn: &TransactionHandle,
    block: &BlockHandle,
) -> bool {
    handle.0.dependents_confirmed(txn.as_txn(), &block)
}
