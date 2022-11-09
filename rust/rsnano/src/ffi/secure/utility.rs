use num::FromPrimitive;

use crate::{core::Networks, remove_temporary_directories, unique_path_for, working_path_for};

#[no_mangle]
pub unsafe extern "C" fn rsn_working_path(network: u16, result: *mut u8, size: usize) -> i32 {
    let network: Networks = match FromPrimitive::from_u16(network) {
        Some(n) => n,
        None => return -1,
    };

    let path = match working_path_for(network) {
        Some(p) => p,
        None => return -1,
    };

    let path_str = path.to_string_lossy();
    let bytes = path_str.as_bytes();
    let result_slice = std::slice::from_raw_parts_mut(result, size);
    result_slice[..bytes.len()].copy_from_slice(bytes);
    bytes.len() as i32
}

#[no_mangle]
pub unsafe extern "C" fn rsn_unique_path(network: u16, result: *mut u8, size: usize) -> i32 {
    let network: Networks = match FromPrimitive::from_u16(network) {
        Some(n) => n,
        None => return -1,
    };

    let path = match unique_path_for(network) {
        Some(p) => p,
        None => return -1,
    };

    let path_str = path.to_string_lossy();
    let bytes = path_str.as_bytes();
    let result_slice = std::slice::from_raw_parts_mut(result, size);
    result_slice[..bytes.len()].copy_from_slice(bytes);
    bytes.len() as i32
}

#[no_mangle]
pub extern "C" fn rsn_remove_temporary_directories() {
    remove_temporary_directories();
}
