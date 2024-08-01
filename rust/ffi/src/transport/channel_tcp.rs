use super::{
    channel::{as_tcp_channel, ChannelHandle},
    EndpointDto,
};
use rsnano_node::transport::Channel;

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_tcp_remote_endpoint(
    handle: *mut ChannelHandle,
    endpoint: *mut EndpointDto,
) {
    (*endpoint) = EndpointDto::from(as_tcp_channel(handle).remote_addr())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_tcp_network_version(handle: *mut ChannelHandle) -> u8 {
    let tcp = as_tcp_channel(handle);
    tcp.network_version()
}
