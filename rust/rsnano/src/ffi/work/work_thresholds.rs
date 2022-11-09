use num_traits::FromPrimitive;
use std::convert::TryFrom;

use crate::{
    core::{BlockDetails, BlockType, Networks, Root, WorkVersion},
    ffi::{
        core::{BlockDetailsDto, BlockHandle},
        StringDto,
    },
    work::WorkThresholds,
};

#[repr(C)]
pub struct WorkThresholdsDto {
    pub epoch_1: u64,
    pub epoch_2: u64,
    pub epoch_2_receive: u64,
    pub base: u64,
    pub entry: u64,
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_thresholds_create(
    dto: *mut WorkThresholdsDto,
    epoch_1: u64,
    epoch_2: u64,
    epoch_2_receive: u64,
) {
    let thresholds = WorkThresholds::new(epoch_1, epoch_2, epoch_2_receive);
    fill_work_thresholds_dto(&mut *dto, &thresholds);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_thresholds_publish_full(dto: *mut WorkThresholdsDto) {
    fill_work_thresholds_dto(&mut *dto, WorkThresholds::publish_full())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_thresholds_publish_beta(dto: *mut WorkThresholdsDto) {
    fill_work_thresholds_dto(&mut *dto, WorkThresholds::publish_beta())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_thresholds_publish_dev(dto: *mut WorkThresholdsDto) {
    fill_work_thresholds_dto(&mut *dto, WorkThresholds::publish_dev())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_thresholds_publish_test(dto: *mut WorkThresholdsDto) {
    fill_work_thresholds_dto(&mut *dto, WorkThresholds::publish_test())
}

#[no_mangle]
pub extern "C" fn rsn_work_thresholds_threshold_entry(
    dto: &WorkThresholdsDto,
    work_version: u8,
    block_type: u8,
) -> u64 {
    let block_type = BlockType::try_from(block_type).unwrap_or(BlockType::Invalid);
    let work_version = WorkVersion::try_from(work_version).unwrap_or(WorkVersion::Unspecified);
    let thresholds = WorkThresholds::from(dto);

    thresholds.threshold_entry(block_type, work_version)
}

#[no_mangle]
pub extern "C" fn rsn_work_thresholds_threshold(
    dto: &WorkThresholdsDto,
    details: &BlockDetailsDto,
) -> u64 {
    let thresholds = WorkThresholds::from(dto);
    let details = match BlockDetails::try_from(details) {
        Ok(d) => d,
        Err(_) => return u64::MAX,
    };

    thresholds.threshold(&details)
}

#[no_mangle]
pub extern "C" fn rsn_work_thresholds_threshold2(
    dto: &WorkThresholdsDto,
    work_version: u8,
    details: &BlockDetailsDto,
) -> u64 {
    let thresholds = WorkThresholds::from(dto);
    let work_version = WorkVersion::try_from(work_version).unwrap_or(WorkVersion::Unspecified);
    let details = match BlockDetails::try_from(details) {
        Ok(d) => d,
        Err(_) => return u64::MAX,
    };

    thresholds.threshold2(work_version, &details)
}

#[no_mangle]
pub extern "C" fn rsn_work_thresholds_threshold_base(
    dto: &WorkThresholdsDto,
    work_version: u8,
) -> u64 {
    let thresholds = WorkThresholds::from(dto);
    let work_version = WorkVersion::try_from(work_version).unwrap_or(WorkVersion::Unspecified);
    thresholds.threshold_base(work_version)
}

#[no_mangle]
pub extern "C" fn rsn_work_thresholds_value(
    dto: &WorkThresholdsDto,
    root: &[u8; 32],
    work: u64,
) -> u64 {
    let thresholds = WorkThresholds::from(dto);
    let root = Root::from_bytes(*root);
    thresholds.value(&root, work)
}

#[no_mangle]
pub extern "C" fn rsn_work_thresholds_normalized_multiplier(
    dto: &WorkThresholdsDto,
    multiplier: f64,
    threshold: u64,
) -> f64 {
    let thresholds = WorkThresholds::from(dto);
    thresholds.normalized_multiplier(multiplier, threshold)
}

#[no_mangle]
pub extern "C" fn rsn_work_thresholds_denormalized_multiplier(
    dto: &WorkThresholdsDto,
    multiplier: f64,
    threshold: u64,
) -> f64 {
    let thresholds = WorkThresholds::from(dto);
    thresholds.denormalized_multiplier(multiplier, threshold)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_thresholds_difficulty(
    dto: &WorkThresholdsDto,
    work_version: u8,
    root: *const u8,
    work: u64,
) -> u64 {
    let work_version = WorkVersion::try_from(work_version).unwrap_or(WorkVersion::Unspecified);
    let root = Root::from_ptr(root);
    let thresholds = WorkThresholds::from(dto);
    thresholds.difficulty(work_version, &root, work)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_thresholds_difficulty_block(
    dto: &WorkThresholdsDto,
    block: *const BlockHandle,
) -> u64 {
    let thresholds = WorkThresholds::from(dto);
    let block = (*block).block.read().unwrap();
    thresholds.difficulty_block(block.as_block())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_thresholds_validate_entry(
    dto: &WorkThresholdsDto,
    work_version: u8,
    root: *const u8,
    work: u64,
) -> bool {
    let work_version = WorkVersion::try_from(work_version).unwrap_or(WorkVersion::Unspecified);
    let root = Root::from_ptr(root);
    let thresholds = WorkThresholds::from(dto);
    thresholds.validate_entry(work_version, &root, work)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_work_thresholds_validate_entry_block(
    dto: &WorkThresholdsDto,
    block: *mut BlockHandle,
) -> bool {
    let thresholds = WorkThresholds::from(dto);
    let lk = (*block).block.read().unwrap();
    thresholds.validate_entry_block(lk.as_block())
}

pub fn fill_work_thresholds_dto(dto: &mut WorkThresholdsDto, thresholds: &WorkThresholds) {
    dto.epoch_1 = thresholds.epoch_1;
    dto.epoch_2 = thresholds.epoch_2;
    dto.epoch_2_receive = thresholds.epoch_2_receive;
    dto.base = thresholds.base;
    dto.entry = thresholds.entry;
}

impl TryFrom<u8> for WorkVersion {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(WorkVersion::Unspecified),
            1 => Ok(WorkVersion::Work1),
            _ => Err(anyhow!("unknown work version")),
        }
    }
}

impl From<WorkThresholdsDto> for WorkThresholds {
    fn from(dto: WorkThresholdsDto) -> Self {
        WorkThresholds::new(dto.epoch_1, dto.epoch_2, dto.epoch_2_receive)
    }
}

impl From<&WorkThresholdsDto> for WorkThresholds {
    fn from(dto: &WorkThresholdsDto) -> Self {
        WorkThresholds::new(dto.epoch_1, dto.epoch_2, dto.epoch_2_receive)
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_network_to_string(network: u16, result: *mut StringDto) {
    (*result) = Networks::from_u16(network).unwrap().as_str().into();
}
