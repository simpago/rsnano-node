use rsnano_core::utils::{BufferWriter, Serialize, Stream};
use serde_derive::Serialize;
use std::{
    fmt::Display,
    net::{Ipv6Addr, SocketAddrV6},
};

use super::MessageVariant;

#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct Keepalive {
    pub peers: [SocketAddrV6; 8],
}

impl Keepalive {
    pub const fn new_test_instance() -> Self {
        Self {
            peers: [
                SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0xffff, 1, 2, 3, 4), 1111, 0, 0),
                SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0xffff, 1, 2, 3, 5), 2222, 0, 0),
                SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0xffff, 1, 2, 3, 6), 3333, 0, 0),
                SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0xffff, 1, 2, 3, 7), 4444, 0, 0),
                SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0xffff, 1, 2, 3, 8), 5555, 0, 0),
                SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0xffff, 1, 2, 3, 9), 6666, 0, 0),
                SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0xffff, 1, 2, 3, 0x10), 7777, 0, 0),
                SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0xffff, 1, 2, 3, 0x11), 8888, 0, 0),
            ],
        }
    }
    pub fn deserialize(stream: &mut impl Stream) -> Option<Self> {
        let mut peers = empty_peers();

        for i in 0..8 {
            let mut addr_buffer = [0u8; 16];
            let mut port_buffer = [0u8; 2];
            stream.read_bytes(&mut addr_buffer, 16).ok()?;
            stream.read_bytes(&mut port_buffer, 2).ok()?;

            let port = u16::from_le_bytes(port_buffer);
            let ip_addr = Ipv6Addr::from(addr_buffer);

            peers[i] = SocketAddrV6::new(ip_addr, port, 0, 0);
        }

        Some(Self { peers })
    }

    pub fn serialized_size() -> usize {
        8 * (16 + 2)
    }
}

impl Default for Keepalive {
    fn default() -> Self {
        Self {
            peers: empty_peers(),
        }
    }
}

impl MessageVariant for Keepalive {}

impl Serialize for Keepalive {
    fn serialize(&self, stream: &mut dyn BufferWriter) {
        for peer in &self.peers {
            let ip_bytes = peer.ip().octets();
            stream.write_bytes_safe(&ip_bytes);

            let port_bytes = peer.port().to_le_bytes();
            stream.write_bytes_safe(&port_bytes);
        }
    }
}

impl Display for Keepalive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for peer in &self.peers {
            write!(f, "\n{}", peer)?;
        }
        Ok(())
    }
}

fn empty_peers() -> [SocketAddrV6; 8] {
    [SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0); 8]
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::{assert_deserializable, Message};

    #[test]
    fn serialize_no_peers() {
        let request = Message::Keepalive(Keepalive::default());
        assert_deserializable(&request);
    }

    #[test]
    fn serialize_peers() {
        let mut keepalive = Keepalive::default();
        keepalive.peers[0] = SocketAddrV6::new(Ipv6Addr::LOCALHOST, 10000, 0, 0);
        let request = Message::Keepalive(keepalive);
        assert_deserializable(&request);
    }

    #[test]
    fn keepalive_with_no_peers_to_string() {
        let keepalive = Message::Keepalive(Default::default());
        let expected = "\n[::]:0\n[::]:0\n[::]:0\n[::]:0\n[::]:0\n[::]:0\n[::]:0\n[::]:0";
        assert_eq!(keepalive.to_string(), expected);
    }

    #[test]
    fn keepalive_string() {
        let mut keepalive = Keepalive::default();
        keepalive.peers[1] = SocketAddrV6::new(Ipv6Addr::LOCALHOST, 45, 0, 0);
        keepalive.peers[2] = SocketAddrV6::new(
            Ipv6Addr::from_str("2001:db8:85a3:8d3:1319:8a2e:370:7348").unwrap(),
            0,
            0,
            0,
        );
        keepalive.peers[3] = SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 65535, 0, 0);
        keepalive.peers[4] =
            SocketAddrV6::new(Ipv6Addr::from_str("::ffff:1.2.3.4").unwrap(), 1234, 0, 0);
        keepalive.peers[5] =
            SocketAddrV6::new(Ipv6Addr::from_str("::ffff:1.2.3.4").unwrap(), 1234, 0, 0);
        keepalive.peers[6] =
            SocketAddrV6::new(Ipv6Addr::from_str("::ffff:1.2.3.4").unwrap(), 1234, 0, 0);
        keepalive.peers[7] =
            SocketAddrV6::new(Ipv6Addr::from_str("::ffff:1.2.3.4").unwrap(), 1234, 0, 0);

        let mut expected = String::new();
        expected.push_str("\n[::]:0");
        expected.push_str("\n[::1]:45");
        expected.push_str("\n[2001:db8:85a3:8d3:1319:8a2e:370:7348]:0");
        expected.push_str("\n[::]:65535");
        expected.push_str("\n[::ffff:1.2.3.4]:1234");
        expected.push_str("\n[::ffff:1.2.3.4]:1234");
        expected.push_str("\n[::ffff:1.2.3.4]:1234");
        expected.push_str("\n[::ffff:1.2.3.4]:1234");

        assert_eq!(keepalive.to_string(), expected);
    }
}
