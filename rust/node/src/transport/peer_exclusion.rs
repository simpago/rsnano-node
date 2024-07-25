#[cfg(test)]
use mock_instant::Instant;
use rsnano_core::utils::{ContainerInfo, ContainerInfoComponent};
#[cfg(not(test))]
use std::time::Instant;
use std::{
    collections::{BTreeMap, HashMap},
    net::{Ipv6Addr, SocketAddrV6},
    time::Duration,
};

/// Manages excluded peers.
/// Peers are excluded for a while if they behave badly
pub struct PeerExclusion {
    ordered_by_date: PeersOrderedByExclusionDate,
    by_ip: HashMap<Ipv6Addr, Peer>,
    max_size: usize,
}

impl PeerExclusion {
    pub fn new() -> Self {
        Self::with_max_size(5000)
    }

    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            ordered_by_date: PeersOrderedByExclusionDate::new(),
            by_ip: HashMap::new(),
            max_size,
        }
    }

    /// Excludes the given `endpoint` for a while. If the endpoint was already
    /// excluded its exclusion duration gets increased.
    /// Returns the new score for the peer.
    pub fn peer_misbehaved(&mut self, endpoint: &SocketAddrV6) -> u64 {
        if let Some(peer) = self.by_ip.get_mut(&endpoint.ip()) {
            let old_exclution_end = peer.exclude_until;
            peer.misbehaved();
            if peer.exclude_until != old_exclution_end {
                self.ordered_by_date
                    .update_exclusion_end(old_exclution_end, peer);
            }
            peer.score
        } else {
            self.clean_old_peers();
            let peer = Peer::new(*endpoint);
            self.insert(&peer);
            peer.score
        }
    }

    pub fn score(&self, endpoint: &SocketAddrV6) -> u64 {
        self.by_ip
            .get(&endpoint.ip())
            .map(|peer| peer.score)
            .unwrap_or_default()
    }

    pub fn contains(&self, endpoint: &SocketAddrV6) -> bool {
        self.by_ip.contains_key(&endpoint.ip())
    }

    pub fn excluded_until(&self, endpoint: &SocketAddrV6) -> Option<Instant> {
        self.by_ip
            .get(&endpoint.ip())
            .map(|item| item.exclude_until)
    }

    /// Checks if an endpoint is currently excluded.
    pub fn is_excluded(&mut self, endpoint: &SocketAddrV6) -> bool {
        self.is_excluded_ip(endpoint.ip())
    }

    pub fn is_excluded_ip(&mut self, ip: &Ipv6Addr) -> bool {
        if let Some(peer) = self.by_ip.get(ip).cloned() {
            if peer.has_expired() {
                self.remove(&peer.address);
            }
            peer.is_excluded()
        } else {
            false
        }
    }

    pub fn remove(&mut self, endpoint: &SocketAddrV6) {
        if let Some(item) = self.by_ip.remove(&endpoint.ip()) {
            self.ordered_by_date
                .remove(&item.address.ip(), item.exclude_until);
        }
    }

    pub fn size(&self) -> usize {
        self.by_ip.len()
    }

    pub fn element_size() -> usize {
        std::mem::size_of::<Peer>()
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
                count: self.size(),
                sizeof_element: Self::element_size(),
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
    exclude_until: Instant,
    address: SocketAddrV6,

    /// gets increased for each bad behaviour
    score: u64,
}

impl Peer {
    fn new(address: SocketAddrV6) -> Self {
        let score = 1;
        Self {
            address,
            exclude_until: Instant::now() + EXCLUDE_TIME_HOURS,
            score,
        }
    }

    fn misbehaved(&mut self) {
        self.score += 1;
        self.exclude_until = Self::exclusion_end(self.score);
    }

    fn exclusion_end(new_score: u64) -> Instant {
        Instant::now() + EXCLUDE_TIME_HOURS * Self::exclusion_duration_factor(new_score)
    }

    fn exclusion_duration_factor(new_score: u64) -> u32 {
        if new_score <= SCORE_LIMIT {
            1
        } else {
            new_score as u32 * 2
        }
    }

    fn is_excluded(&self) -> bool {
        self.score >= SCORE_LIMIT && self.exclude_until > Instant::now()
    }

    fn has_expired(&self) -> bool {
        (self.exclude_until + EXCLUDE_REMOVE_HOURS * self.score as u32) < Instant::now()
    }
}

struct PeersOrderedByExclusionDate(BTreeMap<Instant, Vec<Ipv6Addr>>);

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

    fn update_exclusion_end(&mut self, old_date: Instant, peer: &Peer) {
        self.remove(&peer.address.ip(), old_date);
        self.insert(*peer.address.ip(), peer.exclude_until);
    }

    pub fn insert(&mut self, ip: Ipv6Addr, exclude_until: Instant) {
        let entries = self.0.entry(exclude_until).or_default();
        entries.push(ip);
    }

    pub fn remove(&mut self, ip: &Ipv6Addr, exclude_until: Instant) {
        let entries = self.0.get_mut(&exclude_until).unwrap();
        entries.retain(|x| x != ip);
        if entries.is_empty() {
            self.0.remove(&exclude_until);
        }
    }
}

/// When `SCORE_LIMIT` is reached then a peer will be excluded
const SCORE_LIMIT: u64 = 2;
static EXCLUDE_TIME_HOURS: Duration = Duration::from_secs(60 * 60);
static EXCLUDE_REMOVE_HOURS: Duration = Duration::from_secs(60 * 60 * 24);

#[cfg(test)]
mod tests {
    use super::*;
    use mock_instant::MockClock;
    use std::net::Ipv6Addr;

    #[test]
    fn new_excluded_peers_excludes_nothing() {
        let mut excluded_peers = PeerExclusion::new();
        assert_eq!(excluded_peers.is_excluded(&test_endpoint(1)), false);
        assert_eq!(excluded_peers.is_excluded(&test_endpoint(2)), false);
    }

    #[test]
    fn misbehaving_once_is_allowed() {
        let mut excluded_peers = PeerExclusion::new();
        let endpoint = test_endpoint(1);
        excluded_peers.peer_misbehaved(&endpoint);
        assert_eq!(excluded_peers.is_excluded(&endpoint), false);
    }

    #[test]
    fn misbehaving_twice_leads_to_a_ban() {
        let mut excluded_peers = PeerExclusion::new();
        let endpoint = test_endpoint(1);
        excluded_peers.peer_misbehaved(&endpoint);
        excluded_peers.peer_misbehaved(&endpoint);
        assert_eq!(excluded_peers.is_excluded(&endpoint), true);
        assert_eq!(
            excluded_peers.excluded_until(&endpoint),
            Some(Instant::now() + EXCLUDE_TIME_HOURS)
        );
    }

    #[test]
    fn misbehaving_more_than_twice_increases_exclusion_time() {
        let mut excluded_peers = PeerExclusion::new();
        let endpoint = test_endpoint(1);
        excluded_peers.peer_misbehaved(&endpoint);
        excluded_peers.peer_misbehaved(&endpoint);
        assert_eq!(
            excluded_peers.excluded_until(&endpoint),
            Some(Instant::now() + EXCLUDE_TIME_HOURS)
        );
        excluded_peers.peer_misbehaved(&endpoint);
        assert_eq!(
            excluded_peers.excluded_until(&endpoint),
            Some(Instant::now() + EXCLUDE_TIME_HOURS * 6)
        );
        excluded_peers.peer_misbehaved(&endpoint);
        assert_eq!(
            excluded_peers.excluded_until(&endpoint),
            Some(Instant::now() + EXCLUDE_TIME_HOURS * 8)
        );
    }

    #[test]
    fn remove_oldest_entry() {
        let mut excluded_peers = PeerExclusion::with_max_size(6);
        for i in 0..6 {
            excluded_peers.peer_misbehaved(&test_endpoint(i));
            MockClock::advance(Duration::from_millis(1));
        }
        assert_eq!(excluded_peers.size(), 6);
        excluded_peers.peer_misbehaved(&test_endpoint(6));
        assert_eq!(excluded_peers.size(), 6);
        assert_eq!(excluded_peers.contains(&test_endpoint(0)), false);
        assert_eq!(excluded_peers.contains(&test_endpoint(1)), true);
    }

    #[test]
    fn remove_many_old_entries() {
        let mut excluded_peers = PeerExclusion::with_max_size(2);
        for i in 0..6 {
            excluded_peers.peer_misbehaved(&test_endpoint(i));
            MockClock::advance(Duration::from_millis(1));
        }

        excluded_peers.peer_misbehaved(&test_endpoint(6));
        assert_eq!(excluded_peers.size(), 2);
        assert_eq!(excluded_peers.contains(&test_endpoint(4)), false);
        assert_eq!(excluded_peers.contains(&test_endpoint(5)), true);
        assert_eq!(excluded_peers.contains(&test_endpoint(6)), true);
    }

    fn test_endpoint(i: usize) -> SocketAddrV6 {
        SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, i as u16), 0, 0, 0)
    }
}
