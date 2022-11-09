use std::{slice, sync::Arc};

use crate::{
    core::{Account, Amount},
    ledger::RepWeights,
};

pub struct RepWeightsHandle(Arc<RepWeights>);

impl RepWeightsHandle {
    pub fn new(weights: Arc<RepWeights>) -> *mut RepWeightsHandle {
        Box::into_raw(Box::new(RepWeightsHandle(weights)))
    }
}

#[no_mangle]
pub extern "C" fn rsn_rep_weights_create() -> *mut RepWeightsHandle {
    RepWeightsHandle::new(Arc::new(RepWeights::new()))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_rep_weights_destroy(handle: *mut RepWeightsHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_rep_weights_representation_add(
    handle: *mut RepWeightsHandle,
    source_rep: *const u8,
    amount: *const u8,
) {
    let amount = Amount::from_ptr(amount);
    (*handle)
        .0
        .representation_add(Account::from_ptr(source_rep), amount);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_rep_weights_representation_add_dual(
    handle: *mut RepWeightsHandle,
    source_rep_1: *const u8,
    amount_1: *const u8,
    source_rep_2: *const u8,
    amount_2: *const u8,
) {
    (*handle).0.representation_add_dual(
        Account::from_ptr(source_rep_1),
        Amount::from_ptr(amount_1),
        Account::from_ptr(source_rep_2),
        Amount::from_ptr(amount_2),
    );
}

#[no_mangle]
pub unsafe extern "C" fn rsn_rep_weights_representation_get(
    handle: *mut RepWeightsHandle,
    account: *const u8,
    result: *mut u8,
) {
    let result = slice::from_raw_parts_mut(result, 16);
    let representation = (*handle).0.representation_get(&Account::from_ptr(account));
    result.copy_from_slice(&representation.to_be_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_rep_weights_item_size() -> usize {
    RepWeights::item_size()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_rep_weights_item_count(handle: *const RepWeightsHandle) -> usize {
    (*handle).0.count()
}

#[repr(C)]
pub struct RepAmountItemDto {
    account: [u8; 32],
    amount: [u8; 16],
}

pub struct RepAmountsRawData(Vec<RepAmountItemDto>);

#[repr(C)]
pub struct RepAmountsDto {
    items: *const RepAmountItemDto,
    count: usize,
    pub raw_data: *mut RepAmountsRawData,
}

#[no_mangle]
pub unsafe extern "C" fn rsn_rep_weights_get_rep_amounts(
    handle: *mut RepWeightsHandle,
    result: *mut RepAmountsDto,
) {
    let amounts = (*handle).0.get_rep_amounts();
    let items = amounts
        .iter()
        .map(|(account, amount)| RepAmountItemDto {
            account: *account.as_bytes(),
            amount: amount.to_be_bytes(),
        })
        .collect();
    let raw_data = Box::new(RepAmountsRawData(items));
    (*result).items = raw_data.0.as_ptr();
    (*result).count = raw_data.0.len();
    (*result).raw_data = Box::into_raw(raw_data);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_rep_weights_destroy_amounts_dto(amounts: *mut RepAmountsDto) {
    drop(Box::from_raw((*amounts).raw_data))
}
