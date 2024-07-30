use anyhow::Result;
use rsnano_core::utils::{TomlArrayWriter, TomlWriter};
use std::{
    ffi::{c_char, c_void, CString},
    ptr,
    str::FromStr,
};
use toml::Value;

type TomlPutU64Callback =
    unsafe extern "C" fn(*mut c_void, *const u8, usize, u64, *const u8, usize) -> i32;

type TomlPutI64Callback =
    unsafe extern "C" fn(*mut c_void, *const u8, usize, i64, *const u8, usize) -> i32;

type TomlPutF64Callback =
    unsafe extern "C" fn(*mut c_void, *const u8, usize, f64, *const u8, usize) -> i32;

type TomlPutStrCallback =
    unsafe extern "C" fn(*mut c_void, *const u8, usize, *const u8, usize, *const u8, usize) -> i32;

type TomlPutBoolCallback =
    unsafe extern "C" fn(*mut c_void, *const u8, usize, bool, *const u8, usize) -> i32;

type TomlCreateArrayCallback =
    unsafe extern "C" fn(*mut c_void, *const u8, usize, *const u8, usize) -> *mut c_void;

type TomlDropArrayCallback = unsafe extern "C" fn(*mut c_void);
type TomlArrayPutStrCallback = unsafe extern "C" fn(*mut c_void, *const u8, usize);
type TomlCreateConfigCallback = extern "C" fn() -> *mut c_void;
type TomlDropConfigCallback = extern "C" fn(*mut c_void);
type TomlPutChildCallback = extern "C" fn(*mut c_void, *const u8, usize, *mut c_void);

static mut PUT_U64_CALLBACK: Option<TomlPutU64Callback> = None;
static mut PUT_I64_CALLBACK: Option<TomlPutI64Callback> = None;
static mut PUT_F64_CALLBACK: Option<TomlPutF64Callback> = None;
static mut PUT_STR_CALLBACK: Option<TomlPutStrCallback> = None;
static mut PUT_BOOL_CALLBACK: Option<TomlPutBoolCallback> = None;
static mut CREATE_ARRAY_CALLBACK: Option<TomlCreateArrayCallback> = None;
static mut DROP_ARRAY_CALLBACK: Option<TomlDropArrayCallback> = None;
static mut PUT_ARRAY_STR_CALLBACK: Option<TomlArrayPutStrCallback> = None;
static mut CREATE_CONFIG_CALLBACK: Option<TomlCreateConfigCallback> = None;
static mut DROP_CONFIG_CALLBACK: Option<TomlDropConfigCallback> = None;
static mut PUT_CHILD_CALLBACK: Option<TomlPutChildCallback> = None;

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_toml_create_config(f: TomlCreateConfigCallback) {
    CREATE_CONFIG_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_toml_drop_config(f: TomlDropConfigCallback) {
    DROP_CONFIG_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_toml_put_child(f: TomlPutChildCallback) {
    PUT_CHILD_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_toml_put_u64(f: TomlPutU64Callback) {
    PUT_U64_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_toml_put_i64(f: TomlPutI64Callback) {
    PUT_I64_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_toml_put_f64(f: TomlPutF64Callback) {
    PUT_F64_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_toml_put_str(f: TomlPutStrCallback) {
    PUT_STR_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_toml_put_bool(f: TomlPutBoolCallback) {
    PUT_BOOL_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_toml_create_array(f: TomlCreateArrayCallback) {
    CREATE_ARRAY_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_toml_drop_array(f: TomlDropArrayCallback) {
    DROP_ARRAY_CALLBACK = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_toml_array_put_str(f: TomlArrayPutStrCallback) {
    PUT_ARRAY_STR_CALLBACK = Some(f);
}

pub struct FfiToml {
    handle: *mut c_void,
}

impl FfiToml {
    pub fn new(handle: *mut c_void) -> Self {
        Self { handle }
    }

    /*pub fn write_toml_string(&mut self, toml_string: &str) -> Result<()> {
    let toml_value = Value::from_str(toml_string)?;

    self.write_toml_value("", &toml_value)
    }*/

    pub fn write_toml_string(&mut self, toml_string: &str) -> Result<()> {
        unsafe {
            let c_str = std::ffi::CString::new(toml_string).unwrap();
            std::ptr::copy_nonoverlapping(
                c_str.as_ptr() as *const c_char,
                self.handle as *mut c_char,
                c_str.as_bytes().len() + 1,
            );
        }
        Ok(())
    }

    fn write_toml_value(&mut self, key_prefix: &str, value: &Value) -> Result<()> {
        match value {
            Value::String(s) => self.put_str(key_prefix, s, ""),
            Value::Integer(i) => self.put_i64(key_prefix, *i, ""),
            Value::Float(f) => self.put_f64(key_prefix, *f, ""),
            Value::Boolean(b) => self.put_bool(key_prefix, *b, ""),
            Value::Array(arr) => self.create_array(key_prefix, "", &mut |array_writer| {
                for val in arr {
                    if let Value::String(s) = val {
                        array_writer.push_back_str(s)?;
                    } else {
                        // You can extend this to handle other types of array elements
                        return Err(anyhow::anyhow!("Unsupported array element type"));
                    }
                }
                Ok(())
            }),
            Value::Table(tbl) => {
                for (k, v) in tbl {
                    let full_key = if key_prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{}.{}", key_prefix, k)
                    };
                    self.write_toml_value(&full_key, v)?;
                }
                Ok(())
            }
            _ => Err(anyhow::anyhow!("Unsupported TOML value type")),
        }
    }
}

impl TomlWriter for FfiToml {
    fn put_u16(&mut self, key: &str, value: u16, documentation: &str) -> Result<()> {
        self.put_u64(key, value as u64, documentation)
    }

    fn put_u32(&mut self, key: &str, value: u32, documentation: &str) -> Result<()> {
        self.put_u64(key, value as u64, documentation)
    }

    fn put_u64(&mut self, key: &str, value: u64, documentation: &str) -> Result<()> {
        unsafe {
            match PUT_U64_CALLBACK {
                Some(f) => {
                    if f(
                        self.handle,
                        key.as_ptr(),
                        key.bytes().len(),
                        value,
                        documentation.as_ptr(),
                        documentation.as_bytes().len(),
                    ) == 0
                    {
                        Ok(())
                    } else {
                        Err(anyhow!("PUT_U32_CALLBACK returned error"))
                    }
                }
                None => Err(anyhow!("PUT_U32_CALLBACK not set")),
            }
        }
    }

    fn put_i64(&mut self, key: &str, value: i64, documentation: &str) -> Result<()> {
        unsafe {
            match PUT_I64_CALLBACK {
                Some(f) => {
                    if f(
                        self.handle,
                        key.as_ptr(),
                        key.bytes().len(),
                        value,
                        documentation.as_ptr(),
                        documentation.as_bytes().len(),
                    ) == 0
                    {
                        Ok(())
                    } else {
                        Err(anyhow!("PUT_I64_CALLBACK returned error"))
                    }
                }
                None => Err(anyhow!("PUT_I64_CALLBACK not set")),
            }
        }
    }

    fn put_str(&mut self, key: &str, value: &str, documentation: &str) -> Result<()> {
        unsafe {
            match PUT_STR_CALLBACK {
                Some(f) => {
                    if f(
                        self.handle,
                        key.as_ptr(),
                        key.bytes().len(),
                        value.as_ptr(),
                        value.bytes().len(),
                        documentation.as_ptr(),
                        documentation.as_bytes().len(),
                    ) == 0
                    {
                        Ok(())
                    } else {
                        Err(anyhow!("PUT_STR_CALLBACK returned error"))
                    }
                }
                None => Err(anyhow!("PUT_STR_CALLBACK not set")),
            }
        }
    }

    fn put_bool(&mut self, key: &str, value: bool, documentation: &str) -> Result<()> {
        unsafe {
            match PUT_BOOL_CALLBACK {
                Some(f) => {
                    if f(
                        self.handle,
                        key.as_ptr(),
                        key.bytes().len(),
                        value,
                        documentation.as_ptr(),
                        documentation.as_bytes().len(),
                    ) == 0
                    {
                        Ok(())
                    } else {
                        Err(anyhow!("PUT_BOOL_CALLBACK returned error"))
                    }
                }
                None => Err(anyhow!("PUT_BOOL_CALLBACK not set")),
            }
        }
    }

    fn put_usize(&mut self, key: &str, value: usize, documentation: &str) -> Result<()> {
        self.put_u64(key, value as u64, documentation)
    }

    fn put_f64(&mut self, key: &str, value: f64, documentation: &str) -> Result<()> {
        unsafe {
            match PUT_F64_CALLBACK {
                Some(f) => {
                    if f(
                        self.handle,
                        key.as_ptr(),
                        key.bytes().len(),
                        value,
                        documentation.as_ptr(),
                        documentation.as_bytes().len(),
                    ) == 0
                    {
                        Ok(())
                    } else {
                        Err(anyhow!("PUT_F64_CALLBACK returned error"))
                    }
                }
                None => Err(anyhow!("PUT_F64_CALLBACK not set")),
            }
        }
    }

    fn create_array(
        &mut self,
        key: &str,
        documentation: &str,
        f: &mut dyn FnMut(&mut dyn TomlArrayWriter) -> Result<()>,
    ) -> Result<()> {
        unsafe {
            match CREATE_ARRAY_CALLBACK {
                Some(cb) => {
                    let ptr = cb(
                        self.handle,
                        key.as_ptr(),
                        key.bytes().len(),
                        documentation.as_ptr(),
                        documentation.as_bytes().len(),
                    );

                    if !ptr.is_null() {
                        let mut writer = FfiTomlArrayWriter::new(ptr);
                        f(&mut writer)
                    } else {
                        Err(anyhow!("CREATE_ARRAY_CALLBACK returned error"))
                    }
                }
                None => Err(anyhow!("CREATE_ARRAY_CALLBACK not set")),
            }
        }
    }

    fn put_child(
        &mut self,
        key: &str,
        f: &mut dyn FnMut(&mut dyn TomlWriter) -> Result<()>,
    ) -> Result<()> {
        unsafe {
            let create_config =
                CREATE_CONFIG_CALLBACK.ok_or_else(|| anyhow!("CREATE_CONFIG_CALLBACK not set"))?;
            let drop_config =
                DROP_CONFIG_CALLBACK.ok_or_else(|| anyhow!("DROP_CONFIG_CALLBACK not set"))?;
            let put_child =
                PUT_CHILD_CALLBACK.ok_or_else(|| anyhow!("PUT_CHILD_CALLBACK not set"))?;
            let handle = create_config();
            let mut toml = FfiToml::new(handle);
            let res = f(&mut toml);
            if res.is_err() {
                drop_config(handle)
            }
            res?;
            put_child(self.handle, key.as_ptr(), key.bytes().len(), handle);
            drop_config(handle);
            Ok(())
        }
    }
}

pub struct FfiTomlArrayWriter {
    handle: *mut c_void,
}

impl FfiTomlArrayWriter {
    pub fn new(handle: *mut c_void) -> Self {
        Self { handle }
    }
}

impl TomlArrayWriter for FfiTomlArrayWriter {
    fn push_back_str(&mut self, value: &str) -> Result<()> {
        unsafe {
            match PUT_ARRAY_STR_CALLBACK {
                Some(f) => {
                    f(self.handle, value.as_ptr(), value.bytes().len());
                    Ok(())
                }
                None => Err(anyhow!("PUT_ARRAY_STR_CALLBACK not set")),
            }
        }
    }
}

impl Drop for FfiTomlArrayWriter {
    fn drop(&mut self) {
        unsafe {
            match DROP_ARRAY_CALLBACK {
                Some(f) => {
                    f(self.handle);
                }
                None => panic!("ROP_ARRAY_CALLBACK not set"),
            }
        }
    }
}
