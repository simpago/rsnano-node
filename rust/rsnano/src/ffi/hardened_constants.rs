use crate::core::HardenedConstants;

#[no_mangle]
pub unsafe extern "C" fn rsn_hardened_constants_get(not_an_account: *mut u8, random_128: *mut u8) {
    let not_an_account = std::slice::from_raw_parts_mut(not_an_account, 32);
    let random_128 = std::slice::from_raw_parts_mut(random_128, 16);
    not_an_account.copy_from_slice(HardenedConstants::get().not_an_account.as_bytes());
    random_128.copy_from_slice(&HardenedConstants::get().random_128.to_ne_bytes());
}
