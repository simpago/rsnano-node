use super::EndpointDto;
use rsnano_core::{
    utils::{system_time_as_nanoseconds, system_time_from_nanoseconds, NULL_ENDPOINT},
    Account,
};
use rsnano_node::transport::Channel;
use std::{ops::Deref, sync::Arc, time::SystemTime};

pub struct ChannelHandle(Arc<Channel>);

impl ChannelHandle {
    pub fn new(channel: Arc<Channel>) -> *mut Self {
        Box::into_raw(Box::new(Self(channel)))
    }
}

impl Deref for ChannelHandle {
    type Target = Arc<Channel>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_destroy(handle: *mut ChannelHandle) {
    drop(Box::from_raw(handle));
}

#[no_mangle]
pub extern "C" fn rsn_channel_close(handle: &ChannelHandle) {
    handle.close()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_get_last_packet_received(handle: &ChannelHandle) -> u64 {
    system_time_as_nanoseconds(handle.get_last_packet_received())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_get_last_packet_sent(handle: &ChannelHandle) -> u64 {
    system_time_as_nanoseconds(handle.get_last_packet_sent())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_set_last_packet_sent(handle: &ChannelHandle) {
    handle.set_last_packet_sent(SystemTime::now());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_set_last_packet_sent2(handle: &ChannelHandle, time: u64) {
    handle.set_last_packet_sent(system_time_from_nanoseconds(time));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_get_node_id(handle: &ChannelHandle, result: *mut u8) -> bool {
    match handle.get_node_id() {
        Some(id) => {
            std::slice::from_raw_parts_mut(result, 32).copy_from_slice(id.as_bytes());
            true
        }
        None => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_set_node_id(handle: &ChannelHandle, id: *const u8) {
    handle.set_node_id(Account::from_ptr(id));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_id(handle: &ChannelHandle) -> usize {
    handle.channel_id().as_usize()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_peering_endpoint(
    handle: &ChannelHandle,
    result: *mut EndpointDto,
) {
    (*result) = handle.peering_endpoint().unwrap_or(NULL_ENDPOINT).into()
}
