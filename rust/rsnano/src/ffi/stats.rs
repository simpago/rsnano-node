use std::{
    ffi::{c_void, CStr},
    ops::Deref,
    sync::Arc,
};

use num::FromPrimitive;

use crate::{
    core::messages::MessageType,
    stats::{
        DetailType, Direction, FileWriter, JsonWriter, Stat, StatConfig, StatLogSink, StatType,
    },
};

use super::FfiPropertyTreeWriter;

#[repr(C)]
pub struct StatConfigDto {
    pub sampling_enabled: bool,
    pub capacity: usize,
    pub interval: usize,
    pub log_interval_samples: usize,
    pub log_interval_counters: usize,
    pub log_rotation_count: usize,
    pub log_headers: bool,
    pub log_counters_filename: [u8; 128],
    pub log_counters_filename_len: usize,
    pub log_samples_filename: [u8; 128],
    pub log_samples_filename_len: usize,
}

pub fn fill_stat_config_dto(dto: &mut StatConfigDto, config: &StatConfig) {
    dto.sampling_enabled = config.sampling_enabled;
    dto.capacity = config.capacity;
    dto.interval = config.interval;
    dto.log_interval_samples = config.log_interval_samples;
    dto.log_interval_counters = config.log_interval_counters;
    dto.log_rotation_count = config.log_rotation_count;
    dto.log_headers = config.log_headers;
    let bytes = config.log_counters_filename.as_bytes();
    dto.log_counters_filename[..bytes.len()].copy_from_slice(bytes);
    dto.log_counters_filename_len = bytes.len();
    let bytes = config.log_samples_filename.as_bytes();
    dto.log_samples_filename[..bytes.len()].copy_from_slice(bytes);
    dto.log_samples_filename_len = bytes.len();
}

impl From<&StatConfigDto> for StatConfig {
    fn from(dto: &StatConfigDto) -> Self {
        Self {
            sampling_enabled: dto.sampling_enabled,
            capacity: dto.capacity,
            interval: dto.interval,
            log_interval_samples: dto.log_interval_samples,
            log_interval_counters: dto.log_interval_counters,
            log_rotation_count: dto.log_rotation_count,
            log_headers: dto.log_headers,
            log_counters_filename: String::from_utf8_lossy(
                &dto.log_counters_filename[..dto.log_counters_filename_len],
            )
            .to_string(),
            log_samples_filename: String::from_utf8_lossy(
                &dto.log_samples_filename[..dto.log_samples_filename_len],
            )
            .to_string(),
        }
    }
}

pub struct StatHandle(Arc<Stat>);

impl StatHandle {
    pub fn new(stat: &Arc<Stat>) -> *mut Self {
        Box::into_raw(Box::new(StatHandle(Arc::clone(stat))))
    }
}

impl Deref for StatHandle {
    type Target = Arc<Stat>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_stat_create(config: *const StatConfigDto) -> *mut StatHandle {
    Box::into_raw(Box::new(StatHandle(Arc::new(Stat::new(StatConfig::from(
        &*config,
    ))))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_stat_destroy(handle: *mut StatHandle) {
    drop(Box::from_raw(handle))
}

pub struct StatLogSinkHandle(Box<dyn StatLogSink>);

#[no_mangle]
pub unsafe extern "C" fn rsn_file_writer_create(filename: *const i8) -> *mut StatLogSinkHandle {
    let filename = CStr::from_ptr(filename).to_str().unwrap();
    Box::into_raw(Box::new(StatLogSinkHandle(Box::new(
        FileWriter::new(filename).unwrap(),
    ))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_json_writer_create() -> *mut StatLogSinkHandle {
    Box::into_raw(Box::new(StatLogSinkHandle(Box::new(JsonWriter::new()))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_stat_log_sink_destroy(handle: *mut StatLogSinkHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_stat_log_sink_to_object(
    handle: *mut StatLogSinkHandle,
) -> *mut c_void {
    let obj = (*handle).0.to_object();
    match obj {
        Some(obj) => {
            let x = obj.downcast_ref::<FfiPropertyTreeWriter>().unwrap();
            x.handle
        }
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_stat_type_to_string(key: u8, result: *mut *const u8) -> usize {
    let type_str = StatType::from_u8(key).unwrap().as_str();
    (*result) = type_str.as_ptr();
    type_str.len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_stat_detail_to_string(key: u8, result: *mut *const u8) -> usize {
    let type_str = DetailType::from_u8(key).unwrap().as_str();
    (*result) = type_str.as_ptr();
    type_str.len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_stat_dir_to_string(key: u8, result: *mut *const u8) -> usize {
    let type_str = Direction::from_u8(key).unwrap().as_str();
    (*result) = type_str.as_ptr();
    type_str.len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_stat_stop(handle: *mut StatHandle) {
    (*handle).0.stop();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_stat_add(
    handle: *mut StatHandle,
    stat_type: u8,
    detail: u8,
    dir: u8,
    value: u64,
    detail_only: bool,
) {
    let stat_type = FromPrimitive::from_u8(stat_type).unwrap();
    let detail = FromPrimitive::from_u8(detail).unwrap();
    let dir = FromPrimitive::from_u8(dir).unwrap();
    (*handle)
        .0
        .add(stat_type, detail, dir, value, detail_only)
        .unwrap();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_stat_log_counters(
    handle: *mut StatHandle,
    sink_handle: *mut StatLogSinkHandle,
) {
    let sink = (*sink_handle).0.as_mut();
    (*handle).0.log_counters(sink).unwrap();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_stat_log_samples(
    handle: *mut StatHandle,
    sink_handle: *mut StatLogSinkHandle,
) {
    let sink = (*sink_handle).0.as_mut();
    (*handle).0.log_samples(sink).unwrap();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_stat_last_reset_s(handle: *mut StatHandle) -> u64 {
    (*handle).0.last_reset().as_secs()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_stat_clear(handle: *mut StatHandle) {
    (*handle).0.clear();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_stat_configure(
    handle: *mut StatHandle,
    stat_type: u8,
    detail: u8,
    dir: u8,
    interval: usize,
    capacity: usize,
) {
    let stat_type = FromPrimitive::from_u8(stat_type).unwrap();
    let detail = FromPrimitive::from_u8(detail).unwrap();
    let dir = FromPrimitive::from_u8(dir).unwrap();
    (*handle)
        .0
        .configure(stat_type, detail, dir, interval, capacity);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_stat_count(
    handle: *mut StatHandle,
    stat_type: u8,
    detail: u8,
    dir: u8,
) -> u64 {
    let stat_type = FromPrimitive::from_u8(stat_type).unwrap();
    let detail = FromPrimitive::from_u8(detail).unwrap();
    let dir = FromPrimitive::from_u8(dir).unwrap();
    (*handle).0.count(stat_type, detail, dir)
}

#[no_mangle]
pub extern "C" fn rsn_message_type_to_stat_detail(message_type: u8) -> u8 {
    DetailType::from(MessageType::from_u8(message_type).unwrap()) as u8
}
