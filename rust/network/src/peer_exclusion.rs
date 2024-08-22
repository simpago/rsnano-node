use rsnano_core::utils::{ContainerInfo, ContainerInfoComponent};
use rsnano_nullable_clock::Timestamp;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    mem::size_of,
    net::{Ipv6Addr, SocketAddrV6},
    time::Duration,
};

/// Manages excluded peers.
/// Peers are excluded for a while if they behave badly
pub struct PeerExclusion {
    ordered_by_date: PeersOrderedByExclusionDate,
    by_ip: HashMap<Ipv6Addr, Peer>,
    max_size: usize,
    perma_bans: HashSet<SocketAddrV6>,
}

impl PeerExclusion {
    pub fn new() -> Self {
        Self::with_max_size(5000)
    }

    /// Max size is for misbehaving peers and does not include perma bans
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            ordered_by_date: PeersOrderedByExclusionDate::new(),
            by_ip: HashMap::new(),
            max_size,
            perma_bans: HashSet::new(),
        }
    }

    /// Excludes the given `endpoint` for a while. If the endpoint was already
    /// excluded its exclusion duration gets increased.
    /// Returns the new score for the peer.
    pub fn peer_misbehaved(&mut self, endpoint: &SocketAddrV6, now: Timestamp) -> u64 {
        if let Some(peer) = self.by_ip.get_mut(&endpoint.ip()) {
            let old_exclution_end = peer.exclude_until;
            peer.misbehaved(now);
            if peer.exclude_until != old_exclution_end {
                self.ordered_by_date
                    .update_exclusion_end(old_exclution_end, peer);
            }
            peer.score
        } else {
            self.clean_old_peers();
            let peer = Peer::new(*endpoint, now);
            self.insert(&peer);
            peer.score
        }
    }

    /// Perma bans are used for prohibiting a node to connect to itself.
    pub fn perma_ban(&mut self, peer_addr: SocketAddrV6) {
        self.perma_bans.insert(peer_addr);
    }

    #[allow(dead_code)]
    pub fn contains(&self, endpoint: &SocketAddrV6) -> bool {
        self.by_ip.contains_key(&endpoint.ip()) || self.perma_bans.contains(endpoint)
    }

    #[allow(dead_code)]
    pub fn excluded_until(&self, endpoint: &SocketAddrV6) -> Option<Timestamp> {
        if self.perma_bans.contains(endpoint) {
            Some(Timestamp::MAX)
        } else {
            self.by_ip
                .get(&endpoint.ip())
                .map(|item| item.exclude_until)
        }
    }

    /// Checks if an endpoint is currently excluded.
    pub fn is_excluded(&mut self, peer_addr: &SocketAddrV6, now: Timestamp) -> bool {
        if self.perma_bans.contains(&peer_addr) {
            return true;
        }

        if let Some(peer) = self.by_ip.get(&peer_addr.ip()).cloned() {
            if peer.has_expired(now) {
                self.remove(&peer.address);
            }
            peer.is_excluded(now)
        } else {
            false
        }
    }

    fn remove(&mut self, endpoint: &SocketAddrV6) {
        if let Some(item) = self.by_ip.remove(&endpoint.ip()) {
            self.ordered_by_date
                .remove(&item.address.ip(), item.exclude_until);
        }
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.by_ip.len() + self.perma_bans.len()
    }

    fn clean_old_peers(&mut self) {
        while self.by_ip.len() > 1 && self.by_ip.len() >= self.max_size {
            let ip = self.ordered_by_date.pop().unwrap();
            self.by_ip.remove(&ip);
        }
    }

    fn insert(&mut self, peer: &Peer) {
        self.ordered_by_date
            .insert(*peer.address.ip(), peer.exclude_until);
        self.by_ip.insert(*peer.address.ip(), peer.clone());
    }

    pub fn collect_container_info(&self, name: impl Into<String>) -> ContainerInfoComponent {
        ContainerInfoComponent::Composite(
            name.into(),
            vec![ContainerInfoComponent::Leaf(ContainerInfo {
                name: "peers".to_string(),
                count: self.by_ip.len(),
                sizeof_element: size_of::<Peer>(),
            })],
        )
    }
}

impl Default for PeerExclusion {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a peer and its exclusion status
#[derive(Clone)]
struct Peer {
    exclude_until: Timestamp,
    address: SocketAddrV6,

    /// gets increased for each bad behaviour
    score: u64,
}

impl Peer {
    /// When `SCORE_LIMIT` is reached then a peer will be excluded
    const SCORE_LIMIT: u64 = 2;
    const EXCLUDE_TIME: Duration = Duration::from_secs(60 * 60);
    const EXCLUDE_REMOVE: Duration = Duration::from_secs(60 * 60 * 24);

    fn new(address: SocketAddrV6, now: Timestamp) -> Self {
        let score = 1;
        Self {
            address,
            exclude_until: now + Self::EXCLUDE_TIME,
            score,
        }
    }

    fn misbehaved(&mut self, now: Timestamp) {
        self.score += 1;
        self.exclude_until = Self::exclusion_end(self.score, now);
    }

    fn exclusion_end(new_score: u64, now: Timestamp) -> Timestamp {
        now + Self::EXCLUDE_TIME * Self::exclusion_duration_factor(new_score)
    }

    fn exclusion_duration_factor(new_score: u64) -> u32 {
        if new_score <= Self::SCORE_LIMIT {
            1
        } else {
            new_score as u32 * 2
        }
    }

    fn is_excluded(&self, now: Timestamp) -> bool {
        self.score >= Self::SCORE_LIMIT && self.exclude_until > now
    }

    fn has_expired(&self, now: Timestamp) -> bool {
        (self.exclude_until + Self::EXCLUDE_REMOVE * self.score as u32) < now
    }
}

struct PeersOrderedByExclusionDate(BTreeMap<Timestamp, Vec<Ipv6Addr>>);

impl PeersOrderedByExclusionDate {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    fn pop(&mut self) -> Option<Ipv6Addr> {
        let (&instant, ips) = self.0.iter_mut().next()?;
        let ip = ips.pop().unwrap(); // ips is never empty
        if ips.is_empty() {
            self.0.remove(&instant);
        }
        Some(ip)
    }

    fn update_exclusion_end(&mut self, old_date: Timestamp, peer: &Peer) {
        self.remove(&peer.address.ip(), old_date);
        self.insert(*peer.address.ip(), peer.exclude_until);
    }

    pub fn insert(&mut self, ip: Ipv6Addr, exclude_until: Timestamp) {
        let entries = self.0.entry(exclude_until).or_default();
        entries.push(ip);
    }

    pub fn remove(&mut self, ip: &Ipv6Addr, exclude_until: Timestamp) {
        let entries = self.0.get_mut(&exclude_until).unwrap();
        entries.retain(|x| x != ip);
        if entries.is_empty() {
            self.0.remove(&exclude_until);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv6Addr;

    #[test]
    fn new_excluded_peers_excludes_nothing() {
        let mut peers = PeerExclusion::new();
        assert_eq!(peers.is_excluded(&test_endpoint(1), NOW), false);
        assert_eq!(peers.is_excluded(&test_endpoint(2), NOW), false);
    }

    mod misbehavior {
        use super::*;

        #[test]
        fn misbehaving_once_is_allowed() {
            let mut peers = PeerExclusion::new();
            let endpoint = test_endpoint(1);
            peers.peer_misbehaved(&endpoint, NOW);
            assert_eq!(peers.is_excluded(&endpoint, NOW), false);
        }

        #[test]
        fn misbehaving_twice_leads_to_a_ban() {
            let mut peers = PeerExclusion::new();
            let endpoint = test_endpoint(1);
            peers.peer_misbehaved(&endpoint, NOW);
            peers.peer_misbehaved(&endpoint, NOW);
            assert_eq!(peers.is_excluded(&endpoint, NOW), true);
            assert_eq!(
                peers.excluded_until(&endpoint),
                Some(NOW + Peer::EXCLUDE_TIME)
            );
        }

        #[test]
        fn misbehaving_more_than_twice_increases_exclusion_time() {
            let mut peers = PeerExclusion::new();
            let endpoint = test_endpoint(1);
            peers.peer_misbehaved(&endpoint, NOW);
            peers.peer_misbehaved(&endpoint, NOW);
            peers.peer_misbehaved(&endpoint, NOW);
            assert_eq!(
                peers.excluded_until(&endpoint),
                Some(NOW + Peer::EXCLUDE_TIME * 6)
            );
            peers.peer_misbehaved(&endpoint, NOW);
            assert_eq!(
                peers.excluded_until(&endpoint),
                Some(NOW + Peer::EXCLUDE_TIME * 8)
            );
        }

        #[test]
        fn peer_misbehavior_ignores_port() {
            let mut endpoint1 = test_endpoint(1);
            let mut endpoint2 = endpoint1.clone();
            endpoint1.set_port(100);
            endpoint2.set_port(200);

            let mut peers = PeerExclusion::new();
            peers.peer_misbehaved(&endpoint1, NOW);
            peers.peer_misbehaved(&endpoint2, NOW);

            assert!(peers.is_excluded(&endpoint1, NOW));
            assert!(peers.is_excluded(&endpoint2, NOW));
        }
    }

    mod max_size {
        use super::*;

        #[test]
        fn remove_oldest_entry_when_size_limit_reached() {
            let mut peers = PeerExclusion::with_max_size(6);
            for i in 0..7 {
                peers.peer_misbehaved(&test_endpoint(i), NOW + Duration::from_millis(i as u64));
            }
            assert_eq!(peers.len(), 6);
            assert_eq!(peers.contains(&test_endpoint(0)), false);
            assert_eq!(peers.contains(&test_endpoint(1)), true);
        }

        #[test]
        fn remove_many_old_entries() {
            let mut peers = PeerExclusion::with_max_size(2);
            for i in 0..7 {
                peers.peer_misbehaved(&test_endpoint(i), NOW + Duration::from_millis(i as u64));
            }

            assert_eq!(peers.len(), 2);
            assert_eq!(peers.contains(&test_endpoint(4)), false);
            assert_eq!(peers.contains(&test_endpoint(5)), true);
            assert_eq!(peers.contains(&test_endpoint(6)), true);
        }
    }

    mod perma_bans {
        use super::*;

        #[test]
        fn perma_ban() {
            let mut peers = PeerExclusion::new();
            let endpoint = test_endpoint(1);
            peers.perma_ban(endpoint);
            assert!(peers.is_excluded(&endpoint, NOW));
            assert!(peers.is_excluded(&endpoint, NOW + Duration::from_secs(60 * 60 * 24 * 365)));
            assert_eq!(peers.excluded_until(&endpoint), Some(Timestamp::MAX));
            assert!(peers.contains(&endpoint));
            assert_eq!(peers.len(), 1);
        }
    }

    fn test_endpoint(i: usize) -> SocketAddrV6 {
        SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, i as u16), 0, 0, 0)
    }

    const NOW: Timestamp = Timestamp::new_test_instance();
}
