use std::ffi::c_void;

use crate::utils::Stream;

type WriteU8Callback = unsafe extern "C" fn(*mut c_void, u8) -> i32;
type WriteBytesCallback = unsafe extern "C" fn(*mut c_void, *const u8, usize) -> i32;
type ReadU8Callback = unsafe extern "C" fn(*mut c_void, *mut u8) -> i32;
type ReadBytesCallback = unsafe extern "C" fn(*mut c_void, *mut u8, usize) -> i32;
type InAvailCallback = unsafe extern "C" fn(*mut c_void, *mut i32) -> usize;

static mut WRITE_U8_CALLBACK: Option<WriteU8Callback> = None;
static mut WRITE_BYTES_CALLBACK: Option<WriteBytesCallback> = None;
static mut READ_U8_CALLBACK: Option<ReadU8Callback> = None;
static mut READ_BYTES_CALLBACK: Option<ReadBytesCallback> = None;
static mut IN_AVAIL_CALLBACK: Option<InAvailCallback> = None;

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_write_u8(f: WriteU8Callback) {
    WRITE_U8_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_write_bytes(f: WriteBytesCallback) {
    WRITE_BYTES_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_read_u8(f: ReadU8Callback) {
    READ_U8_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_read_bytes(f: ReadBytesCallback) {
    READ_BYTES_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_in_avail(f: InAvailCallback) {
    IN_AVAIL_CALLBACK = Some(f);
}

pub struct FfiStream {
    stream_handle: *mut c_void,
}

impl FfiStream {
    pub fn new(stream_handle: *mut c_void) -> Self {
        Self { stream_handle }
    }
}

impl Stream for FfiStream {
    fn write_u8(&mut self, value: u8) -> anyhow::Result<()> {
        unsafe {
            match WRITE_U8_CALLBACK {
                Some(f) => {
                    let result = f(self.stream_handle, value);

                    if result == 0 {
                        Ok(())
                    } else {
                        Err(anyhow!("callback returned error"))
                    }
                }
                None => Err(anyhow!("WRITE_U8_CALLBACK missing")),
            }
        }
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        unsafe {
            match WRITE_BYTES_CALLBACK {
                Some(f) => {
                    if f(self.stream_handle, bytes.as_ptr(), bytes.len()) == 0 {
                        Ok(())
                    } else {
                        Err(anyhow!("callback returned error"))
                    }
                }
                None => Err(anyhow!("WRITE_32_BYTES_CALLBACK missing")),
            }
        }
    }

    fn read_u8(&mut self) -> anyhow::Result<u8> {
        unsafe {
            match READ_U8_CALLBACK {
                Some(f) => {
                    let mut value = 0u8;
                    let raw_value = &mut value as *mut u8;
                    if f(self.stream_handle, raw_value) == 0 {
                        Ok(value)
                    } else {
                        Err(anyhow!("callback returned error"))
                    }
                }
                None => Err(anyhow!("READ_U8_CALLBACK missing")),
            }
        }
    }

    fn read_bytes(&mut self, buffer: &mut [u8], len: usize) -> anyhow::Result<()> {
        unsafe {
            match READ_BYTES_CALLBACK {
                Some(f) => {
                    if f(self.stream_handle, buffer.as_mut_ptr(), len) == 0 {
                        Ok(())
                    } else {
                        Err(anyhow!("callback returned error"))
                    }
                }
                None => Err(anyhow!("READ_BYTES_CALLBACK missing")),
            }
        }
    }

    fn in_avail(&mut self) -> anyhow::Result<usize> {
        unsafe {
            match IN_AVAIL_CALLBACK {
                Some(f) => {
                    let mut error = 0;
                    let avail = f(self.stream_handle, &mut error);
                    if error == 0 {
                        Ok(avail)
                    } else {
                        Err(anyhow!("in_avail returned error"))
                    }
                }
                None => Err(anyhow!("IN_AVAIL_CALLBACK missing")),
            }
        }
    }
}
