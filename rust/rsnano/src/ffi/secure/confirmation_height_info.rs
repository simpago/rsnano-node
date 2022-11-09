use std::ffi::c_void;

use crate::utils::{Deserialize, Serialize};
use crate::{
    core::{BlockHash, ConfirmationHeightInfo},
    ffi::utils::FfiStream,
};

#[repr(C)]
pub struct ConfirmationHeightInfoDto {
    pub height: u64,
    pub frontier: [u8; 32],
}

#[no_mangle]
pub unsafe extern "C" fn rsn_confirmation_height_info_create(
    result: *mut ConfirmationHeightInfoDto,
) {
    let info = ConfirmationHeightInfo::default();
    *result = info.into();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_confirmation_height_info_create2(
    height: u64,
    hash: *const u8,
    result: *mut ConfirmationHeightInfoDto,
) {
    let hash = BlockHash::from_ptr(hash);
    let info = ConfirmationHeightInfo::new(height, hash);
    *result = info.into();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_confirmation_height_info_serialize(
    info: *const ConfirmationHeightInfoDto,
    stream: *mut c_void,
) -> bool {
    let info = ConfirmationHeightInfo::from(&*info);
    let mut stream = FfiStream::new(stream);
    info.serialize(&mut stream).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_confirmation_height_info_deserialize(
    info: *mut ConfirmationHeightInfoDto,
    stream: *mut c_void,
) -> bool {
    let mut stream = FfiStream::new(stream);
    match ConfirmationHeightInfo::deserialize(&mut stream) {
        Ok(i) => {
            *info = i.into();
            true
        }
        Err(_) => false,
    }
}

impl From<ConfirmationHeightInfo> for ConfirmationHeightInfoDto {
    fn from(info: ConfirmationHeightInfo) -> Self {
        ConfirmationHeightInfoDto {
            height: info.height,
            frontier: *info.frontier.as_bytes(),
        }
    }
}

impl From<&ConfirmationHeightInfoDto> for ConfirmationHeightInfo {
    fn from(dto: &ConfirmationHeightInfoDto) -> Self {
        ConfirmationHeightInfo {
            height: dto.height,
            frontier: BlockHash::from_bytes(dto.frontier),
        }
    }
}
