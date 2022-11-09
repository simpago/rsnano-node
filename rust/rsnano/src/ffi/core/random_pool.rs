use std::slice;

use rand::{thread_rng, Rng};

#[no_mangle]
pub extern "C" fn rsn_random_pool_generate_word32(min: u32, max: u32) -> u32 {
    thread_rng().gen_range(min..=max)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_random_pool_generate_block(output: *mut u8, len: usize) {
    let bytes = slice::from_raw_parts_mut(output, len);
    thread_rng().fill(bytes);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_random_pool_generate_byte() -> u8 {
    thread_rng().gen()
}
