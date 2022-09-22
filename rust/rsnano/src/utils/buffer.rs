use std::{
    ffi::c_void,
    ops::Deref,
    sync::{Arc, Mutex},
};

pub struct BufferHandle(Arc<Mutex<Vec<u8>>>);

impl BufferHandle {
    pub fn new(buf: Arc<Mutex<Vec<u8>>>) -> *mut BufferHandle {
        Box::into_raw(Box::new(BufferHandle(buf)))
    }
}

impl Deref for BufferHandle {
    type Target = Arc<Mutex<Vec<u8>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub extern "C" fn rsn_buffer_create(len: usize) -> *mut BufferHandle {
    Box::into_raw(Box::new(BufferHandle(Arc::new(Mutex::new(vec![0; len])))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_buffer_destroy(handle: *mut BufferHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_buffer_data(handle: *mut BufferHandle) -> *mut u8 {
    let ptr = (*handle).0.lock().unwrap().as_ptr();
    std::mem::transmute::<*const u8, *mut u8>(ptr)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_buffer_len(handle: *mut BufferHandle) -> usize {
    (*handle).0.lock().unwrap().len()
}

pub trait BufferWrapper {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn handle(&self) -> *mut c_void;
}
