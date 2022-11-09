use crate::{core::messages::Message, ffi::utils::FfiStream};
use std::ffi::c_void;

use super::{
    create_message_handle, create_message_handle2, downcast_message, downcast_message_mut,
    MessageHandle, MessageHeaderHandle,
};
use crate::{
    core::{
        messages::{BulkPullAccount, BulkPullAccountFlags},
        Account, Amount,
    },
    ffi::{copy_account_bytes, copy_amount_bytes, NetworkConstantsDto},
};
use num_traits::FromPrimitive;

#[no_mangle]
pub unsafe extern "C" fn rsn_message_bulk_pull_account_create(
    constants: *mut NetworkConstantsDto,
) -> *mut MessageHandle {
    create_message_handle(constants, BulkPullAccount::new)
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
pub unsafe extern "C" fn rsn_message_bulk_pull_account_serialize(
    handle: *mut MessageHandle,
    stream: *mut c_void,
) -> bool {
    let mut stream = FfiStream::new(stream);
    downcast_message::<BulkPullAccount>(handle)
        .serialize(&mut stream)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_bulk_pull_account_deserialize(
    handle: *mut MessageHandle,
    stream: *mut c_void,
) -> bool {
    let mut stream = FfiStream::new(stream);
    downcast_message_mut::<BulkPullAccount>(handle)
        .deserialize(&mut stream)
        .is_ok()
}
