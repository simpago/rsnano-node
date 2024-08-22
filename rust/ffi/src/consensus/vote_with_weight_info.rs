use rsnano_core::{
    utils::system_time_from_nanoseconds, Amount, BlockHash, PublicKey, VoteWithWeightInfo,
};
use std::{ops::Deref, time::UNIX_EPOCH};

#[repr(C)]
pub struct VoteWithWeightInfoDto {
    pub representative: [u8; 32],
    pub time_ns: u64,
    pub timestamp: u64,
    pub hash: [u8; 32],
    pub weight: [u8; 16],
}

impl From<&VoteWithWeightInfo> for VoteWithWeightInfoDto {
    fn from(value: &VoteWithWeightInfo) -> Self {
        Self {
            representative: *value.representative.as_bytes(),
            time_ns: value.time.duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64,
            timestamp: value.timestamp,
            hash: *value.hash.as_bytes(),
            weight: value.weight.to_be_bytes(),
        }
    }
}

impl From<&VoteWithWeightInfoDto> for VoteWithWeightInfo {
    fn from(value: &VoteWithWeightInfoDto) -> Self {
        Self {
            representative: PublicKey::from_bytes(value.representative),
            time: system_time_from_nanoseconds(value.time_ns),
            timestamp: value.timestamp,
            hash: BlockHash::from_bytes(value.hash),
            weight: Amount::from_be_bytes(value.weight),
        }
    }
}

pub struct VoteWithWeightInfoVecHandle(pub Vec<VoteWithWeightInfo>);

impl VoteWithWeightInfoVecHandle {
    pub fn new(votes: Vec<VoteWithWeightInfo>) -> *mut Self {
        Box::into_raw(Box::new(Self(votes)))
    }
}

impl Deref for VoteWithWeightInfoVecHandle {
    type Target = Vec<VoteWithWeightInfo>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_vote_with_weight_info_vec_destroy(
    handle: *mut VoteWithWeightInfoVecHandle,
) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub extern "C" fn rsn_vote_with_weight_info_vec_len(handle: &VoteWithWeightInfoVecHandle) -> usize {
    handle.0.len()
}

#[no_mangle]
pub extern "C" fn rsn_vote_with_weight_info_vec_get(
    handle: &VoteWithWeightInfoVecHandle,
    index: usize,
    result: &mut VoteWithWeightInfoDto,
) {
    *result = (&handle.0[index]).into()
}
