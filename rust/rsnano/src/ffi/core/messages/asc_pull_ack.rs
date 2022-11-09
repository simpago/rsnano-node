use super::{
    create_message_handle, create_message_handle2, downcast_message, downcast_message_mut,
    message_handle_clone, MessageHandle, MessageHeaderHandle,
};
use crate::{
    core::{
        messages::{
            AccountInfoAckPayload, AscPullAck, AscPullAckPayload, BlocksAckPayload, Message,
        },
        Account, BlockHash,
    },
    ffi::{
        core::{copy_block_array_dto, BlockArrayDto, BlockHandle},
        utils::FfiStream,
        NetworkConstantsDto,
    },
};
use std::{
    borrow::Borrow,
    ffi::c_void,
    sync::{Arc, RwLock},
};

#[no_mangle]
pub unsafe extern "C" fn rsn_message_asc_pull_ack_create(
    constants: *mut NetworkConstantsDto,
) -> *mut MessageHandle {
    create_message_handle(constants, AscPullAck::new)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_asc_pull_ack_create2(
    header: *mut MessageHeaderHandle,
) -> *mut MessageHandle {
    create_message_handle2(header, AscPullAck::with_header)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_asc_pull_ack_clone(
    handle: *mut MessageHandle,
) -> *mut MessageHandle {
    message_handle_clone::<AscPullAck>(handle)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_asc_pull_ack_set_id(handle: *mut MessageHandle, id: u64) {
    downcast_message_mut::<AscPullAck>(handle).id = id;
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_asc_pull_ack_get_id(handle: *mut MessageHandle) -> u64 {
    downcast_message::<AscPullAck>(handle).id
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_asc_pull_ack_pull_type(handle: *mut MessageHandle) -> u8 {
    downcast_message::<AscPullAck>(handle).payload_type() as u8
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_asc_pull_ack_size(header: *mut MessageHeaderHandle) -> usize {
    AscPullAck::serialized_size(&*header)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_asc_pull_ack_deserialize(
    handle: *mut MessageHandle,
    stream: *mut c_void,
) -> bool {
    let mut stream = FfiStream::new(stream);
    downcast_message_mut::<AscPullAck>(handle)
        .deserialize(&mut stream)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_asc_pull_ack_serialize(
    handle: *mut MessageHandle,
    stream: *mut c_void,
) -> bool {
    let mut stream = FfiStream::new(stream);
    downcast_message::<AscPullAck>(handle)
        .serialize(&mut stream)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_asc_pull_ack_payload_type(handle: *mut MessageHandle) -> u8 {
    downcast_message::<AscPullAck>(handle).payload_type() as u8
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_asc_pull_ack_payload_blocks(
    handle: *mut MessageHandle,
    blocks: *mut BlockArrayDto,
) {
    match downcast_message::<AscPullAck>(handle).payload() {
        AscPullAckPayload::Blocks(blks) => {
            let list = blks
                .blocks
                .iter()
                .map(|b| Arc::new(RwLock::new(b.clone())))
                .collect();
            copy_block_array_dto(list, blocks)
        }
        _ => panic!("not a blocks payload"),
    }
}

#[repr(C)]
pub struct AccountInfoAckPayloadDto {
    pub account: [u8; 32],
    pub account_open: [u8; 32],
    pub account_head: [u8; 32],
    pub account_block_count: u64,
    pub account_conf_frontier: [u8; 32],
    pub account_conf_height: u64,
}

impl From<&AccountInfoAckPayload> for AccountInfoAckPayloadDto {
    fn from(payload: &AccountInfoAckPayload) -> Self {
        Self {
            account: *payload.account.as_bytes(),
            account_open: *payload.account_open.as_bytes(),
            account_head: *payload.account_head.as_bytes(),
            account_block_count: payload.account_block_count,
            account_conf_frontier: *payload.account_conf_frontier.as_bytes(),
            account_conf_height: payload.account_conf_height,
        }
    }
}

impl From<&AccountInfoAckPayloadDto> for AccountInfoAckPayload {
    fn from(dto: &AccountInfoAckPayloadDto) -> Self {
        Self {
            account: Account::from_bytes(dto.account),
            account_open: BlockHash::from_bytes(dto.account_open),
            account_head: BlockHash::from_bytes(dto.account_head),
            account_block_count: dto.account_block_count,
            account_conf_frontier: BlockHash::from_bytes(dto.account_conf_frontier),
            account_conf_height: dto.account_conf_height,
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_asc_pull_ack_payload_account_info(
    handle: *mut MessageHandle,
    result: *mut AccountInfoAckPayloadDto,
) {
    match downcast_message::<AscPullAck>(handle).payload() {
        AscPullAckPayload::AccountInfo(account_info) => (*result) = account_info.into(),
        _ => panic!("not an account_info payload"),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_asc_pull_ack_request_blocks(
    handle: *mut MessageHandle,
    blocks: *const *const BlockHandle,
    count: usize,
) {
    let blocks = std::slice::from_raw_parts(blocks, count);
    let payload = BlocksAckPayload {
        blocks: blocks
            .iter()
            .map(|&b| (*b).block.read().unwrap().clone())
            .collect(),
    };
    downcast_message_mut::<AscPullAck>(handle)
        .request_blocks(payload)
        .unwrap();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_asc_pull_ack_request_account_info(
    handle: *mut MessageHandle,
    payload: *const AccountInfoAckPayloadDto,
) {
    let payload = (*payload).borrow().into();
    downcast_message_mut::<AscPullAck>(handle)
        .request_account_info(payload)
        .unwrap();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_asc_pull_ack_request_invalid(handle: *mut MessageHandle) {
    downcast_message_mut::<AscPullAck>(handle).request_invalid();
}
