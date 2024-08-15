use super::{channel::ChannelHandle, EndpointDto};

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_tcp_remote_endpoint(
    handle: &ChannelHandle,
    endpoint: *mut EndpointDto,
) {
    (*endpoint) = EndpointDto::from(handle.remote_addr())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_tcp_network_version(handle: &ChannelHandle) -> u8 {
    handle.protocol_version()
}
