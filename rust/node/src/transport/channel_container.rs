use super::{Channel, ChannelDirection, ChannelId, ChannelMode};
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
    by_channel_id: HashMap<ChannelId, Arc<Channel>>,
    by_endpoint: HashMap<SocketAddrV6, Vec<ChannelId>>,
    sequential: Vec<ChannelId>,
    by_bootstrap_attempt: BTreeMap<SystemTime, Vec<ChannelId>>,
    by_network_version: BTreeMap<u8, Vec<ChannelId>>,
    by_ip_address: HashMap<Ipv6Addr, Vec<ChannelId>>,
    by_subnet: HashMap<Ipv6Addr, Vec<ChannelId>>,
}

impl ChannelContainer {
    pub const ELEMENT_SIZE: usize = std::mem::size_of::<Channel>();

    pub fn insert(&mut self, channel: Arc<Channel>) -> bool {
        let id = channel.channel_id();
        if self.by_channel_id.contains_key(&id) {
            panic!("Channel already in collection!");
        }

        self.sequential.push(id);
        self.by_bootstrap_attempt
            .entry(channel.get_last_bootstrap_attempt())
            .or_default()
            .push(id);
        self.by_network_version
            .entry(channel.protocol_version())
            .or_default()
            .push(id);
        self.by_ip_address
            .entry(channel.ipv4_address_or_ipv6_subnet())
            .or_default()
            .push(id);
        self.by_subnet
            .entry(channel.subnetwork())
            .or_default()
            .push(id);
        self.by_endpoint
            .entry(channel.remote_addr())
            .or_default()
            .push(id);
        self.by_channel_id.insert(id, channel);
        true
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<Channel>> {
        self.by_channel_id.values().filter(|c| c.is_alive())
    }

    pub fn iter_by_last_bootstrap_attempt(&self) -> impl Iterator<Item = &Arc<Channel>> {
        self.by_bootstrap_attempt
            .iter()
            .flat_map(|(_, ids)| ids.iter().map(|id| self.by_channel_id.get(id).unwrap()))
            .filter(|c| c.is_alive())
    }

    pub fn len(&self) -> usize {
        self.by_channel_id.len()
    }

    pub fn count_by_mode(&self, mode: ChannelMode) -> usize {
        self.by_channel_id
            .values()
            .filter(|c| c.mode() == mode && c.is_alive())
            .count()
    }

    fn remove_by_id(&mut self, id: ChannelId) -> Option<Arc<Channel>> {
        if let Some(channel) = self.by_channel_id.remove(&id) {
            self.sequential.retain(|x| *x != id); // todo: linear search is slow?

            remove_from_btree(
                &mut self.by_bootstrap_attempt,
                &channel.get_last_bootstrap_attempt(),
                id,
            );
            remove_from_btree(
                &mut self.by_network_version,
                &channel.protocol_version(),
                id,
            );
            remove_from_hashmap(&mut self.by_endpoint, &channel.remote_addr(), id);
            remove_from_hashmap(
                &mut self.by_ip_address,
                &channel.ipv4_address_or_ipv6_subnet(),
                id,
            );
            remove_from_hashmap(&mut self.by_subnet, &channel.subnetwork(), id);
            Some(channel)
        } else {
            None
        }
    }

    pub fn get_by_remote_addr(&self, remote_addr: &SocketAddrV6) -> Vec<&Arc<Channel>> {
        self.by_endpoint
            .get(remote_addr)
            .map(|ids| {
                ids.iter()
                    .map(|id| self.by_channel_id.get(id).unwrap())
                    .filter(|c| c.is_alive())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_by_peering_addr(&self, peering_addr: &SocketAddrV6) -> Vec<&Arc<Channel>> {
        // TODO use a hashmap?
        self.by_channel_id
            .values()
            .filter(|c| c.peering_endpoint().as_ref() == Some(peering_addr) && c.is_alive())
            .collect()
    }

    pub fn get_by_id(&self, id: ChannelId) -> Option<&Arc<Channel>> {
        self.by_channel_id.get(&id)
    }

    pub fn get_by_node_id(&self, node_id: &PublicKey) -> Option<&Arc<Channel>> {
        self.by_channel_id
            .values()
            .filter(|c| c.get_node_id() == Some(*node_id) && c.is_alive())
            .next()
    }

    pub fn set_last_bootstrap_attempt(&mut self, channel_id: ChannelId, attempt_time: SystemTime) {
        if let Some(channel) = self.by_channel_id.get(&channel_id) {
            let old_time = channel.get_last_bootstrap_attempt();
            channel.set_last_bootstrap_attempt(attempt_time);
            remove_from_btree(&mut self.by_bootstrap_attempt, &old_time, channel_id);
            self.by_bootstrap_attempt
                .entry(attempt_time)
                .or_default()
                .push(channel_id);
        }
    }

    pub fn count_by_ip(&self, ip: &Ipv6Addr) -> usize {
        self.by_ip_address
            .get(ip)
            .map(|channel_ids| channel_ids.len())
            .unwrap_or_default()
    }

    pub fn count_by_direction(&self, direction: ChannelDirection) -> usize {
        self.by_channel_id
            .values()
            .filter(|c| c.direction() == direction && c.is_alive())
            .count()
    }

    pub fn count_by_subnet(&self, subnet: &Ipv6Addr) -> usize {
        self.by_subnet
            .get(subnet)
            .map(|ids| ids.len())
            .unwrap_or_default()
    }

    pub fn clear(&mut self) {
        self.by_endpoint.clear();
        self.sequential.clear();
        self.by_bootstrap_attempt.clear();
        self.by_network_version.clear();
        self.by_ip_address.clear();
        self.by_subnet.clear();
        self.by_channel_id.clear();
    }

    pub fn close_idle_channels(&mut self, cutoff: SystemTime) {
        for entry in self.iter() {
            if entry.get_last_packet_sent() < cutoff {
                debug!(remote_addr = ?entry.remote_addr(), channel_id = %entry.channel_id(), mode = ?entry.mode(), "Closing idle channel");
                entry.close();
            }
        }
    }

    /// Removes dead channels and returns their channel ids
    pub fn remove_dead(&mut self) -> Vec<ChannelId> {
        let dead_channels: Vec<_> = self
            .by_channel_id
            .values()
            .filter(|c| !c.is_alive())
            .cloned()
            .collect();

        for channel in &dead_channels {
            debug!("Removing dead channel: {}", channel.remote_addr());
            self.remove_by_id(channel.channel_id());
        }

        dead_channels.iter().map(|c| c.channel_id()).collect()
    }

    pub fn close_old_protocol_versions(&mut self, min_version: u8) {
        while let Some((version, channel_ids)) = self.by_network_version.first_key_value() {
            if *version < min_version {
                for id in channel_ids {
                    if let Some(channel) = self.by_channel_id.get(id) {
                        debug!(channel_id = %id, peer_addr = ?channel.remote_addr(), version, min_version,
                            "Closing channel with old protocol version",
                        );
                        channel.close();
                    }
                }
            } else {
                break;
            }
        }
    }
}

fn remove_from_hashmap<K>(tree: &mut HashMap<K, Vec<ChannelId>>, key: &K, id: ChannelId)
where
    K: Ord + Hash,
{
    let channel_ids = tree.get_mut(key).unwrap();
    if channel_ids.len() > 1 {
        channel_ids.retain(|x| *x != id);
    } else {
        tree.remove(key);
    }
}

fn remove_from_btree<K: Ord>(tree: &mut BTreeMap<K, Vec<ChannelId>>, key: &K, id: ChannelId) {
    let channel_ids = tree.get_mut(key).unwrap();
    if channel_ids.len() > 1 {
        channel_ids.retain(|x| *x != id);
    } else {
        tree.remove(key);
    }
}
