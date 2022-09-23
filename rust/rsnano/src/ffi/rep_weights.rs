use crate::RepWeights;

pub struct RepWeightsHandle(RepWeights);

#[no_mangle]
pub extern "C" fn rsn_rep_weights_create() -> *mut RepWeightsHandle {
    Box::into_raw(Box::new(RepWeightsHandle(RepWeights::new())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_rep_weights_destroy(handle: *mut RepWeightsHandle) {
    drop(Box::from_raw(handle))
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
            account: account.to_bytes(),
            amount: amount.to_be_bytes(),
        })
        .collect();
    let raw_data = Box::new(RepAmountsRawData(items));
    (*result).count = raw_data.0.len();
    (*result).items = raw_data.0.as_ptr();
    (*result).raw_data = Box::into_raw(raw_data);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_rep_weights_destroy_amounts_dto(amounts: *mut RepAmountsDto) {
    drop(Box::from_raw((*amounts).raw_data))
}
