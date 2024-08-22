use crate::utils::FfiStream;
use num_traits::FromPrimitive;
use rsnano_core::{
    utils::{Deserialize, FixedSizeSerialize},
    AccountInfo, Amount, BlockHash, Epoch, PublicKey,
};
use std::{
    ffi::c_void,
    ops::{Deref, DerefMut},
};

pub struct AccountInfoHandle(pub AccountInfo);

impl Deref for AccountInfoHandle {
    type Target = AccountInfo;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AccountInfoHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_account_info_create(
    head: *const u8,
    rep: *const u8,
    open_block: *const u8,
    balance: *const u8,
    modified: u64,
    block_count: u64,
    epoch: u8,
) -> *mut AccountInfoHandle {
    Box::into_raw(Box::new(AccountInfoHandle(AccountInfo {
        head: BlockHash::from_ptr(head),
        representative: PublicKey::from_ptr(rep),
        open_block: BlockHash::from_ptr(open_block),
        balance: Amount::from_ptr(balance),
        modified,
        block_count,
        epoch: Epoch::from_u8(epoch).unwrap(),
    })))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_account_info_destroy(handle: *mut AccountInfoHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_account_info_clone(
    handle: *mut AccountInfoHandle,
) -> *mut AccountInfoHandle {
    Box::into_raw(Box::new(AccountInfoHandle((*handle).0.clone())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_account_info_deserialize(
    handle: *mut AccountInfoHandle,
    stream: *mut c_void,
) -> bool {
    if let Ok(info) = AccountInfo::deserialize(&mut FfiStream::new(stream)) {
        (*handle).0 = info;
        true
    } else {
        false
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_account_info_values(
    handle: *mut AccountInfoHandle,
    values: *mut AccountInfoDto,
) {
    let info = &(*handle).0;
    let result = &mut (*values);
    result.head = *info.head.as_bytes();
    result.representative = *info.representative.as_bytes();
    result.open_block = *info.open_block.as_bytes();
    result.balance = info.balance.to_be_bytes();
    result.modified = info.modified;
    result.block_count = info.block_count;
    result.epoch = info.epoch as u8;
}

#[repr(C)]
pub struct AccountInfoDto {
    pub head: [u8; 32],
    pub representative: [u8; 32],
    pub open_block: [u8; 32],
    pub balance: [u8; 16],
    pub modified: u64,
    pub block_count: u64,
    pub epoch: u8,
}

#[no_mangle]
pub unsafe extern "C" fn rsn_account_info_equals(
    handle: *mut AccountInfoHandle,
    other: *mut AccountInfoHandle,
) -> bool {
    (*handle).0.eq(&(*other).0)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_account_info_db_size() -> usize {
    AccountInfo::serialized_size()
}
