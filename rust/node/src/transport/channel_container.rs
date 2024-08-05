use super::{ChannelDirection, ChannelEnum, ChannelId, ChannelMode};
use rsnano_core::PublicKey;
use std::{
    collections::{BTreeMap, HashMap},
    hash::Hash,
    net::{Ipv6Addr, SocketAddrV6},
    sync::Arc,
    time::SystemTime,
};
use tracing::debug;

/// Keeps track of all connected channels
#[derive(Default)]
pub struct ChannelContainer {
    by_endpoint: HashMap<SocketAddrV6, Arc<ChannelEnum>>,
    by_random_access: Vec<SocketAddrV6>,
    by_bootstrap_attempt: BTreeMap<SystemTime, Vec<SocketAddrV6>>,
    by_network_version: BTreeMap<u8, Vec<SocketAddrV6>>,
    by_ip_address: HashMap<Ipv6Addr, Vec<SocketAddrV6>>,
    by_subnet: HashMap<Ipv6Addr, Vec<SocketAddrV6>>,
    by_id: HashMap<ChannelId, SocketAddrV6>,
}

impl ChannelContainer {
    pub const ELEMENT_SIZE: usize = std::mem::size_of::<ChannelEnum>();

    pub fn insert(&mut self, channel: Arc<ChannelEnum>) -> bool {
        let endpoint = channel.remote_addr();
        if self.by_endpoint.contains_key(&endpoint) {
            return false;
        }

        self.by_random_access.push(endpoint);
        self.by_bootstrap_attempt
            .entry(channel.get_last_bootstrap_attempt())
            .or_default()
            .push(endpoint);
        self.by_network_version
            .entry(channel.network_version())
            .or_default()
            .push(endpoint);
        self.by_ip_address
            .entry(channel.ipv4_address_or_ipv6_subnet())
            .or_default()
            .push(endpoint);
        self.by_subnet
            .entry(channel.subnetwork())
            .or_default()
            .push(endpoint);
        self.by_id.insert(channel.channel_id(), endpoint);
        self.by_endpoint.insert(channel.remote_addr(), channel);
        true
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<ChannelEnum>> {
        self.by_endpoint.values()
    }

    pub fn iter_by_last_bootstrap_attempt(&self) -> impl Iterator<Item = &Arc<ChannelEnum>> {
        self.by_bootstrap_attempt
            .iter()
            .flat_map(|(_, v)| v.iter().map(|ep| self.by_endpoint.get(ep).unwrap()))
    }

    pub fn len(&self) -> usize {
        self.by_endpoint.len()
    }

    pub fn count_by_mode(&self, mode: ChannelMode) -> usize {
        self.by_endpoint
            .values()
            .filter(|i| i.mode() == mode)
            .count()
    }

    pub fn remove_by_endpoint(&mut self, endpoint: &SocketAddrV6) -> Option<Arc<ChannelEnum>> {
        if let Some(channel) = self.by_endpoint.remove(endpoint) {
            self.by_random_access.retain(|x| x != endpoint); // todo: linear search is slow?

            remove_endpoint_btree(
                &mut self.by_bootstrap_attempt,
                &channel.get_last_bootstrap_attempt(),
                endpoint,
            );
            remove_endpoint_btree(
                &mut self.by_network_version,
                &channel.network_version(),
                endpoint,
            );
            remove_endpoint_map(
                &mut self.by_ip_address,
                &channel.ipv4_address_or_ipv6_subnet(),
                endpoint,
            );
            remove_endpoint_map(&mut self.by_subnet, &channel.subnetwork(), endpoint);
            self.by_id.remove(&channel.channel_id());
            Some(channel.clone())
        } else {
            None
        }
    }

    pub fn get_by_remote_addr(&self, remote_addr: &SocketAddrV6) -> Option<&Arc<ChannelEnum>> {
        self.by_endpoint.get(remote_addr)
    }

    pub fn get_by_peering_addr(&self, peering_addr: &SocketAddrV6) -> Option<&Arc<ChannelEnum>> {
        // TODO use a hashmap?
        self.by_endpoint
            .values()
            .find(|i| i.peering_endpoint().as_ref() == Some(peering_addr))
    }

    pub fn get_by_id(&self, id: ChannelId) -> Option<&Arc<ChannelEnum>> {
        self.by_id.get(&id).and_then(|ep| self.by_endpoint.get(ep))
    }

    pub fn get_by_node_id(&self, node_id: &PublicKey) -> Option<&Arc<ChannelEnum>> {
        self.by_endpoint
            .values()
            .filter(|i| i.get_node_id() == Some(*node_id))
            .next()
    }

    pub fn set_last_bootstrap_attempt(
        &mut self,
        endpoint: &SocketAddrV6,
        attempt_time: SystemTime,
    ) {
        if let Some(channel) = self.by_endpoint.get(endpoint) {
            let old_time = channel.get_last_bootstrap_attempt();
            channel.set_last_bootstrap_attempt(attempt_time);
            remove_endpoint_btree(
                &mut self.by_bootstrap_attempt,
                &old_time,
                &channel.remote_addr(),
            );
            self.by_bootstrap_attempt
                .entry(attempt_time)
                .or_default()
                .push(*endpoint);
        }
    }

    pub fn count_by_ip(&self, ip: &Ipv6Addr) -> usize {
        self.by_ip_address
            .get(ip)
            .map(|endpoints| endpoints.len())
            .unwrap_or_default()
    }

    pub fn count_by_direction(&self, direction: ChannelDirection) -> usize {
        self.by_endpoint
            .values()
            .filter(|entry| entry.direction() == direction)
            .count()
    }

    pub fn count_by_subnet(&self, subnet: &Ipv6Addr) -> usize {
        self.by_subnet
            .get(subnet)
            .map(|endpoints| endpoints.len())
            .unwrap_or_default()
    }

    pub fn clear(&mut self) {
        self.by_endpoint.clear();
        self.by_random_access.clear();
        self.by_bootstrap_attempt.clear();
        self.by_network_version.clear();
        self.by_ip_address.clear();
        self.by_subnet.clear();
        self.by_id.clear();
    }

    pub fn close_idle_channels(&mut self, cutoff: SystemTime) {
        for entry in self.iter() {
            if entry.get_last_packet_sent() < cutoff {
                debug!("Closing idle channel: {}", entry.remote_addr());
                entry.close();
            }
        }
    }

    /// Removes dead channels and returns their channel ids
    pub fn remove_dead(&mut self) -> Vec<ChannelId> {
        let dead_channels: Vec<_> = self
            .by_endpoint
            .values()
            .filter(|c| !c.is_alive())
            .cloned()
            .collect();

        for channel in &dead_channels {
            debug!("Removing dead channel: {}", channel.remote_addr());
            self.remove_by_endpoint(&channel.remote_addr());
        }

        dead_channels.iter().map(|c| c.channel_id()).collect()
    }

    pub fn close_old_protocol_versions(&mut self, min_version: u8) {
        while let Some((version, endpoints)) = self.by_network_version.first_key_value() {
            if *version < min_version {
                for ep in endpoints {
                    debug!(
                        "Closing channel with old protocol version: {} (channels version: {}, min version: {})",
                        ep, version, min_version
                    );
                    if let Some(entry) = self.by_endpoint.get(ep) {
                        entry.close();
                    }
                }
            } else {
                break;
            }
        }
    }
}

fn remove_endpoint_btree<K: Ord>(
    tree: &mut BTreeMap<K, Vec<SocketAddrV6>>,
    key: &K,
    endpoint: &SocketAddrV6,
) {
    let endpoints = tree.get_mut(key).unwrap();
    if endpoints.len() > 1 {
        endpoints.retain(|x| x != endpoint);
    } else {
        tree.remove(key);
    }
}

fn remove_endpoint_map<K: Eq + PartialEq + Hash>(
    map: &mut HashMap<K, Vec<SocketAddrV6>>,
    key: &K,
    endpoint: &SocketAddrV6,
) {
    let endpoints = map.get_mut(key).unwrap();
    if endpoints.len() > 1 {
        endpoints.retain(|x| x != endpoint);
    } else {
        map.remove(key);
    }
}
