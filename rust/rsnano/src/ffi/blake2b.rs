use anyhow::Result;
use std::ffi::c_void;

use crate::utils::Blake2b;

type Blake2BInitCallback = unsafe extern "C" fn(*mut c_void, usize) -> i32;
type Blake2BUpdateCallback = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> i32;
type Blake2BFinalCallback = unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> i32;

static mut INIT_CALLBACK: Option<Blake2BInitCallback> = None;
static mut UPDATE_CALLBACK: Option<Blake2BUpdateCallback> = None;
static mut FINAL_CALLBACK: Option<Blake2BFinalCallback> = None;

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_blake2b_init(f: Blake2BInitCallback) {
    INIT_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_blake2b_update(f: Blake2BUpdateCallback) {
    UPDATE_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_blake2b_final(f: Blake2BFinalCallback) {
    FINAL_CALLBACK = Some(f);
}

pub struct FfiBlake2b {
    state: *mut c_void,
}

impl FfiBlake2b {
    pub fn new(state: *mut c_void) -> Self {
        Self { state }
    }
}

impl Blake2b for FfiBlake2b {
    fn init(&mut self, outlen: usize) -> Result<()> {
        unsafe {
            match INIT_CALLBACK {
                Some(f) => {
                    let result = f(self.state, outlen);
                    if result == 0 {
                        Ok(())
                    } else {
                        Err(anyhow!("init returned {}", result))
                    }
                }
                None => Err(anyhow!("INIT_CALLBACK not provided")),
            }
        }
    }

    fn update(&mut self, bytes: &[u8]) -> Result<()> {
        unsafe {
            match UPDATE_CALLBACK {
                Some(f) => {
                    let result = f(self.state, bytes.as_ptr() as *const c_void, bytes.len());
                    if result == 0 {
                        Ok(())
                    } else {
                        Err(anyhow!("update returned {}", result))
                    }
                }
                None => Err(anyhow!("UPDATE_CALLBACK not provided")),
            }
        }
    }

    fn finalize(&mut self, out: &mut [u8]) -> Result<()> {
        unsafe {
            match FINAL_CALLBACK {
                Some(f) => {
                    let result = f(self.state, out.as_mut_ptr() as *mut c_void, out.len());
                    if result == 0 {
                        Ok(())
                    } else {
                        Err(anyhow!("final returned {}", result))
                    }
                }
                None => Err(anyhow!("FINAL_CALLBACK not provided")),
            }
        }
    }
}
