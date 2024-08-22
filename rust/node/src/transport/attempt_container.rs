use super::ChannelDirection;
use crate::utils::{ipv4_address_or_ipv6_subnet, map_address_to_subnetwork};
use rsnano_nullable_clock::Timestamp;
use std::{
    collections::HashMap,
    net::{Ipv6Addr, SocketAddrV6},
    time::Duration,
};

struct Entry {
    endpoint: SocketAddrV6,
    address: Ipv6Addr,
    subnetwork: Ipv6Addr,
    start: Timestamp,
    direction: ChannelDirection,
}

impl Entry {
    fn new(endpoint: SocketAddrV6, direction: ChannelDirection, start: Timestamp) -> Self {
        Self {
            endpoint,
            address: ipv4_address_or_ipv6_subnet(endpoint.ip()),
            subnetwork: map_address_to_subnetwork(endpoint.ip()),
            start,
            direction,
        }
    }
}

/// Keeps track of running connection attempts
#[derive(Default)]
pub struct AttemptContainer {
    by_endpoint: HashMap<SocketAddrV6, Entry>,
    by_address: HashMap<Ipv6Addr, Vec<SocketAddrV6>>,
    by_subnetwork: HashMap<Ipv6Addr, Vec<SocketAddrV6>>,
}

impl AttemptContainer {
    pub fn insert(
        &mut self,
        endpoint: SocketAddrV6,
        direction: ChannelDirection,
        start: Timestamp,
    ) -> bool {
        if self.by_endpoint.contains_key(&endpoint) {
            return false;
        }

        let attempt = Entry::new(endpoint, direction, start);
        self.by_address
            .entry(attempt.address)
            .or_default()
            .push(attempt.endpoint);
        self.by_subnetwork
            .entry(attempt.subnetwork)
            .or_default()
            .push(attempt.endpoint);
        self.by_endpoint.insert(attempt.endpoint, attempt);
        true
    }

    pub fn remove(&mut self, endpoint: &SocketAddrV6) {
        if let Some(attempt) = self.by_endpoint.remove(endpoint) {
            let by_address = self.by_address.get_mut(&attempt.address).unwrap();
            if by_address.len() > 1 {
                by_address.retain(|x| x != endpoint);
            } else {
                self.by_address.remove(&attempt.address);
            }

            let by_subnet = self.by_subnetwork.get_mut(&attempt.subnetwork).unwrap();
            if by_subnet.len() > 1 {
                by_subnet.retain(|x| x != endpoint);
            } else {
                self.by_subnetwork.remove(&attempt.subnetwork);
            }
        }
    }

    pub fn count_by_subnetwork(&self, subnet: &Ipv6Addr) -> usize {
        // TODO use map_address_to_subnetwork
        match self.by_subnetwork.get(subnet) {
            Some(entries) => entries.len(),
            None => 0,
        }
    }

    pub fn count_by_address(&self, address: &Ipv6Addr) -> usize {
        // TODO use ipv4_address_or_ipv6_subnet!
        match self.by_address.get(address) {
            Some(entries) => entries.len(),
            None => 0,
        }
    }

    pub fn len(&self) -> usize {
        self.by_endpoint.len()
    }

    pub fn purge(&mut self, now: Timestamp, timeout: Duration) {
        while let Some((time, endpoint)) = self.get_oldest() {
            if now - time < timeout {
                return;
            }

            self.remove(&endpoint);
        }
    }

    fn get_oldest(&self) -> Option<(Timestamp, SocketAddrV6)> {
        self.by_endpoint
            .values()
            .filter(|i| i.direction == ChannelDirection::Outbound)
            .min_by_key(|i| i.start)
            .map(|i| (i.start, i.endpoint))
    }

    pub const ELEMENT_SIZE: usize = std::mem::size_of::<Entry>();
}
