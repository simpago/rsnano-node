mod async_runtime;
mod blake2b;
mod json;
mod processing_queue;
mod thread_pool;
mod timer;
mod timer_thread;
mod toml;

pub use crate::utils::timer::{NullTimer, Timer, TimerStrategy, TimerWrapper};
pub use async_runtime::AsyncRuntime;
use blake2::{
    digest::{Update, VariableOutput},
    Blake2bVar,
};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV6};

mod hardened_constants;
pub use hardened_constants::HardenedConstants;

pub use blake2b::*;
pub use json::*;
pub use processing_queue::*;
pub use thread_pool::*;
pub use timer_thread::*;
pub use toml::*;

mod long_running_transaction_logger;
pub use long_running_transaction_logger::LongRunningTransactionLogger;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ErrorCode {
    pub val: i32,
    pub category: u8,
}

pub mod error_category {
    pub const GENERIC: u8 = 0;
    pub const SYSTEM: u8 = 1;
}

impl Default for ErrorCode {
    fn default() -> Self {
        Self {
            val: 0,
            category: error_category::SYSTEM,
        }
    }
}

impl ErrorCode {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn is_ok(&self) -> bool {
        !self.is_err()
    }

    pub fn is_err(&self) -> bool {
        self.val != 0
    }

    pub fn not_supported() -> Self {
        ErrorCode {
            val: 95,
            category: error_category::GENERIC,
        }
    }

    pub fn no_buffer_space() -> Self {
        ErrorCode {
            val: 105,
            category: error_category::GENERIC,
        }
    }

    pub fn host_unreachable() -> Self {
        ErrorCode {
            val: 113,
            category: error_category::GENERIC,
        }
    }

    pub fn fault() -> Self {
        ErrorCode {
            val: 14,
            category: error_category::GENERIC,
        }
    }
}

pub fn ip_address_hash_raw(address: &Ipv6Addr, port: u16) -> u64 {
    let address_bytes = address.octets();
    let mut hasher = Blake2bVar::new(8).unwrap();
    hasher.update(&HardenedConstants::get().random_128.to_be_bytes());
    if port != 0 {
        hasher.update(&port.to_ne_bytes());
    }
    hasher.update(&address_bytes);
    let mut result_bytes = [0; 8];
    hasher.finalize_variable(&mut result_bytes).unwrap();
    u64::from_ne_bytes(result_bytes)
}

pub fn ipv4_address_or_ipv6_subnet(input: &Ipv6Addr) -> Ipv6Addr {
    if is_ipv4_mapped(input) {
        input.clone()
    } else {
        // Assuming /48 subnet prefix for IPv6 as it's relatively easy to acquire such a /48 address range
        first_ipv6_subnet_address(input, 48)
    }
}

pub fn map_address_to_subnetwork(input: &Ipv6Addr) -> Ipv6Addr {
    const IPV6_SUBNET_PREFIX_LENGTH: u8 = 32; // Equivalent to network prefix /32.
    const IPV4_SUBNET_PREFIX_LENGTH: u8 = (128 - 32) + 24; // Limits for /24 IPv4 subnetwork (we're using mapped IPv4 to IPv6 addresses, hence (128 - 32))
    if is_ipv4_mapped(input) {
        first_ipv6_subnet_address(input, IPV4_SUBNET_PREFIX_LENGTH)
    } else {
        first_ipv6_subnet_address(input, IPV6_SUBNET_PREFIX_LENGTH)
    }
}

pub fn first_ipv6_subnet_address(input: &Ipv6Addr, prefix_bits: u8) -> Ipv6Addr {
    fill_remaining_bits(input, prefix_bits, 0)
}

pub fn last_ipv6_subnet_address(input: &Ipv6Addr, prefix_bits: u8) -> Ipv6Addr {
    fill_remaining_bits(input, prefix_bits, 0xFF)
}

pub fn fill_remaining_bits(input: &Ipv6Addr, prefix_bits: u8, filler: u8) -> Ipv6Addr {
    debug_assert_eq!(prefix_bits % 8, 0);
    let index = (prefix_bits / 8) as usize;
    let mut octets = input.octets();
    octets[index..].fill(filler);
    Ipv6Addr::from(octets)
}

pub fn is_ipv4_or_v4_mapped_address(address: &IpAddr) -> bool {
    match address {
        IpAddr::V4(_) => true,
        IpAddr::V6(ip) => is_ipv4_mapped(ip),
    }
}

pub fn is_ipv4_mapped(input: &Ipv6Addr) -> bool {
    matches!(
        input.octets(),
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, _, _, _, _]
    )
}
pub fn into_ipv6_socket_address(input: SocketAddr) -> SocketAddrV6 {
    match input {
        SocketAddr::V4(a) => SocketAddrV6::new(a.ip().to_ipv6_mapped(), a.port(), 0, 0),
        SocketAddr::V6(a) => a,
    }
}

pub fn into_ipv6_address(input: IpAddr) -> Ipv6Addr {
    match input {
        IpAddr::V4(ip) => ip.to_ipv6_mapped(),
        IpAddr::V6(ip) => ip,
    }
}

pub fn reserved_address(endpoint: &SocketAddrV6, allow_local_peers: bool) -> bool {
    const RFC1700_MIN: Ipv6Addr = Ipv4Addr::new(0, 0, 0, 0).to_ipv6_mapped();
    const RFC1700_MAX: Ipv6Addr = Ipv4Addr::new(0, 0xff, 0xff, 0xff).to_ipv6_mapped();
    const RFC1918_1_MIN: Ipv6Addr = Ipv4Addr::new(0x0a, 0, 0, 0).to_ipv6_mapped();
    const RFC1918_1_MAX: Ipv6Addr = Ipv4Addr::new(0x0a, 0xff, 0xff, 0xff).to_ipv6_mapped();
    const RFC1918_2_MIN: Ipv6Addr = Ipv4Addr::new(0xac, 0x10, 0x00, 0x00).to_ipv6_mapped();
    const RFC1918_2_MAX: Ipv6Addr = Ipv4Addr::new(0xac, 0x1f, 0xff, 0xff).to_ipv6_mapped();
    const RFC1918_3_MIN: Ipv6Addr = Ipv4Addr::new(0xc0, 0xa8, 0x00, 0x00).to_ipv6_mapped();
    const RFC1918_3_MAX: Ipv6Addr = Ipv4Addr::new(0xc0, 0xa8, 0xff, 0xff).to_ipv6_mapped();
    const RFC6598_MIN: Ipv6Addr = Ipv4Addr::new(0x64, 0x40, 0x00, 0x00).to_ipv6_mapped();
    const RFC6598_MAX: Ipv6Addr = Ipv4Addr::new(0x64, 0x7f, 0xff, 0xff).to_ipv6_mapped();
    const RFC5737_1_MIN: Ipv6Addr = Ipv4Addr::new(0xc0, 0x00, 0x02, 0x00).to_ipv6_mapped();
    const RFC5737_1_MAX: Ipv6Addr = Ipv4Addr::new(0xc0, 0x00, 0x02, 0xff).to_ipv6_mapped();
    const RFC5737_2_MIN: Ipv6Addr = Ipv4Addr::new(0xc6, 0x33, 0x64, 0x00).to_ipv6_mapped();
    const RFC5737_2_MAX: Ipv6Addr = Ipv4Addr::new(0xc6, 0x33, 0x64, 0xff).to_ipv6_mapped();
    const RFC5737_3_MIN: Ipv6Addr = Ipv4Addr::new(0xcb, 0x00, 0x71, 0x00).to_ipv6_mapped();
    const RFC5737_3_MAX: Ipv6Addr = Ipv4Addr::new(0xcb, 0x00, 0x71, 0xff).to_ipv6_mapped();
    const IPV4_MULTICAST_MIN: Ipv6Addr = Ipv4Addr::new(0xe0, 0x00, 0x00, 0x00).to_ipv6_mapped();
    const IPV4_MULTICAST_MAX: Ipv6Addr = Ipv4Addr::new(0xef, 0xff, 0xff, 0xff).to_ipv6_mapped();
    const RFC6890_MIN: Ipv6Addr = Ipv4Addr::new(0xf0, 0x00, 0x00, 0x00).to_ipv6_mapped();
    const RFC6890_MAX: Ipv6Addr = Ipv4Addr::new(0xff, 0xff, 0xff, 0xff).to_ipv6_mapped();

    const RFC6666_MIN: Ipv6Addr = Ipv6Addr::new(0x100, 0, 0, 0, 0, 0, 0, 0);
    const RFC6666_MAX: Ipv6Addr = Ipv6Addr::new(0x100u16, 0, 0, 0, 0xffff, 0xffff, 0xffff, 0xffff);
    const RFC3849_MIN: Ipv6Addr = Ipv6Addr::new(0x2001u16, 0xdb8, 0, 0, 0, 0, 0, 0);
    const RFC3849_MAX: Ipv6Addr = Ipv6Addr::new(
        0x2001u16, 0xdb8, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff,
    );
    const RFC4193_MIN: Ipv6Addr = Ipv6Addr::new(0xfc00u16, 0, 0, 0, 0, 0, 0, 0);
    const RFC4193_MAX: Ipv6Addr = Ipv6Addr::new(
        0xfd00u16, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff,
    );
    const IPV6_MULTICAST_MIN: Ipv6Addr = Ipv6Addr::new(0xff00u16, 0, 0, 0, 0, 0, 0, 0);
    const IPV6_MULTICAST_MAX: Ipv6Addr = Ipv6Addr::new(
        0xff00u16, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff,
    );

    if endpoint.port() == 0 {
        return true;
    }

    let ip = *endpoint.ip();
    if (ip >= RFC1700_MIN && ip <= RFC1700_MAX)
        || (ip >= RFC5737_1_MIN && ip <= RFC5737_1_MAX)
        || (ip >= RFC5737_2_MIN && ip <= RFC5737_2_MAX)
        || (ip >= RFC5737_3_MIN && ip <= RFC5737_3_MAX)
        || (ip >= IPV4_MULTICAST_MIN && ip <= IPV4_MULTICAST_MAX)
        || (ip >= RFC6890_MIN && ip <= RFC6890_MAX)
        || (ip >= RFC6666_MIN && ip <= RFC6666_MAX)
        || (ip >= RFC3849_MIN && ip <= RFC3849_MAX)
        || (ip >= IPV6_MULTICAST_MIN && ip <= IPV6_MULTICAST_MAX)
    {
        return true;
    }

    if !allow_local_peers {
        if (ip >= RFC1918_1_MIN && ip <= RFC1918_1_MAX)
            || (ip >= RFC1918_2_MIN && ip <= RFC1918_2_MAX)
            || (ip >= RFC1918_3_MIN && ip <= RFC1918_3_MAX)
            || (ip >= RFC6598_MIN && ip <= RFC6598_MAX)
            || (ip >= RFC4193_MIN && ip <= RFC4193_MAX)
        {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_ipv6_subnet_address() {
        let address = Ipv6Addr::new(
            0xa41d, 0xb7b2, 0x8298, 0xcf45, 0x672e, 0xbd1a, 0xe7fb, 0xf713,
        );
        let expected = Ipv6Addr::new(0xa41d, 0xb7b2, 0, 0, 0, 0, 0, 0);
        assert_eq!(first_ipv6_subnet_address(&address, 32), expected);
    }

    #[test]
    fn test_last_ipv6_subnet_address() {
        let address = Ipv6Addr::new(
            0xa41d, 0xb7b2, 0x8298, 0xcf45, 0x672e, 0xbd1a, 0xe7fb, 0xf713,
        );
        let expected = Ipv6Addr::new(
            0xa41d, 0xb7b2, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff,
        );
        assert_eq!(last_ipv6_subnet_address(&address, 32), expected);
    }

    #[test]
    fn reserved_adresses() {
        //valid
        assert_eq!(
            reserved_address(&"[2001::]:1".parse().unwrap(), false),
            false
        );

        // 0 port
        assert_eq!(
            reserved_address(&"[2001::]:0".parse().unwrap(), false),
            true
        );

        //loopback
        assert_eq!(reserved_address(&"[::1]:1".parse().unwrap(), false), false);

        //private network
        assert_eq!(
            reserved_address(&"[::ffff:10.0.0.0]:1".parse().unwrap(), false),
            true
        );
        assert_eq!(
            reserved_address(&"[::ffff:10.0.0.0]:1".parse().unwrap(), true),
            false
        );
    }

    #[test]
    fn ipv6_bind_subnetwork() {
        // IPv6 address within the same /48 subnet should return the network prefix
        let address = "a41d:b7b2:8298:cf45:672e:bd1a:e7fb:f713".parse().unwrap();
        let subnet = ipv4_address_or_ipv6_subnet(&address);
        assert_eq!(subnet, "a41d:b7b2:8298::".parse::<Ipv6Addr>().unwrap());
    }

    #[test]
    fn ipv4_subnetwork() {
        // IPv4 mapped as IPv6 address should return the original IPv4 address
        let address = "::ffff:192.168.1.1".parse().unwrap();
        let subnet = ipv4_address_or_ipv6_subnet(&address);
        assert_eq!(address, subnet);
    }

    #[test]
    fn network_range_ipv6() {
        let address = "a719:0f12:536e:d88a:1331:ba53:4598:04e5".parse().unwrap();
        let subnet = map_address_to_subnetwork(&address);
        assert_eq!(subnet, "a719:0f12::".parse::<Ipv6Addr>().unwrap());
    }

    #[test]
    fn network_range_ipv4() {
        // Default settings test
        let address = "::ffff:80.67.148.225".parse().unwrap();
        let subnet = map_address_to_subnetwork(&address);
        assert_eq!(subnet, "::ffff:80.67.148.0".parse::<Ipv6Addr>().unwrap());
    }
}
