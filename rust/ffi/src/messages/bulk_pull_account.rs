use crate::{copy_account_bytes, copy_amount_bytes, NetworkConstantsDto, StringDto};
use rsnano_node::messages::{BulkPullAccount, BulkPullAccountFlags};

use super::{
    create_message_handle2, create_message_handle3, downcast_message, downcast_message_mut,
    MessageHandle, MessageHeaderHandle,
};
use num_traits::FromPrimitive;
use rsnano_core::{Account, Amount};

#[no_mangle]
pub unsafe extern "C" fn rsn_message_bulk_pull_account_create(
    constants: *mut NetworkConstantsDto,
) -> *mut MessageHandle {
    create_message_handle3(constants, BulkPullAccount::new)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_bulk_pull_account_create2(
    header: *mut MessageHeaderHandle,
) -> *mut MessageHandle {
    create_message_handle2(header, BulkPullAccount::with_header)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_bulk_pull_account_clone(
    other: *mut MessageHandle,
) -> *mut MessageHandle {
    MessageHandle::from_message(downcast_message::<BulkPullAccount>(other).clone())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_bulk_pull_account_account(
    handle: *mut MessageHandle,
    account: *mut u8,
) {
    copy_account_bytes(downcast_message::<BulkPullAccount>(handle).account, account);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_bulk_pull_account_set_account(
    handle: *mut MessageHandle,
    account: *const u8,
) {
    downcast_message_mut::<BulkPullAccount>(handle).account = Account::from_ptr(account);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_bulk_pull_account_minimum_amount(
    handle: *mut MessageHandle,
    amount: *mut u8,
) {
    copy_amount_bytes(
        downcast_message::<BulkPullAccount>(handle).minimum_amount,
        amount,
    );
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_bulk_pull_account_set_minimum_amount(
    handle: *mut MessageHandle,
    amount: *const u8,
) {
    downcast_message_mut::<BulkPullAccount>(handle).minimum_amount = Amount::from_ptr(amount);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_bulk_pull_account_flags(handle: *mut MessageHandle) -> u8 {
    downcast_message::<BulkPullAccount>(handle).flags as u8
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_bulk_pull_account_set_flags(
    handle: *mut MessageHandle,
    flags: u8,
) {
    downcast_message_mut::<BulkPullAccount>(handle).flags =
        BulkPullAccountFlags::from_u8(flags).unwrap();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_bulk_pull_account_size() -> usize {
    BulkPullAccount::serialized_size()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_bulk_pull_account_to_string(
    handle: *mut MessageHandle,
    result: *mut StringDto,
) {
    (*result) = downcast_message_mut::<BulkPullAccount>(handle)
        .to_string()
        .into();
}
