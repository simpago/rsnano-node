use std::{
    net::{Ipv6Addr, SocketAddrV6},
    sync::Arc,
    time::SystemTime,
};

use rsnano_core::Account;

use crate::{
    transport::{Channel, ChannelEnum, ChannelTcp, Socket, TcpServer},
    utils::{ipv4_address_or_ipv6_subnet, map_address_to_subnetwork},
};

pub struct ChannelTcpWrapper {
    pub channel: Arc<ChannelEnum>,
    pub response_server: Option<Arc<TcpServer>>,
}

impl ChannelTcpWrapper {
    pub fn new(channel: Arc<ChannelEnum>, response_server: Option<Arc<TcpServer>>) -> Self {
        Self {
            channel,
            response_server,
        }
    }

    pub fn tcp_channel(&self) -> &ChannelTcp {
        match self.channel.as_ref() {
            ChannelEnum::Tcp(tcp) => tcp,
            _ => panic!("not a tcp channel"),
        }
    }

    pub fn endpoint(&self) -> SocketAddrV6 {
        self.tcp_channel().remote_endpoint()
    }

    pub fn last_packet_sent(&self) -> SystemTime {
        self.channel.get_last_packet_sent()
    }

    pub fn last_bootstrap_attempt(&self) -> SystemTime {
        self.channel.get_last_bootstrap_attempt()
    }

    pub fn socket(&self) -> &Arc<Socket> {
        &self.tcp_channel().socket
    }

    pub fn node_id(&self) -> Option<Account> {
        self.channel.get_node_id()
    }

    pub fn network_version(&self) -> u8 {
        self.tcp_channel().network_version()
    }

    pub fn ip_address(&self) -> Ipv6Addr {
        ipv4_address_or_ipv6_subnet(self.endpoint().ip())
    }

    pub fn subnetwork(&self) -> Ipv6Addr {
        map_address_to_subnetwork(self.endpoint().ip())
    }
}