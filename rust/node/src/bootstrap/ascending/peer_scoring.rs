use super::BootstrapAscendingConfig;
use crate::{
    config::NetworkConstants,
    transport::{ChannelEnum, TrafficType},
};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Weak},
};

/// Container for tracking and scoring peers with respect to bootstrapping
pub(crate) struct PeerScoring {
    scoring: Scoring,
    network_constants: NetworkConstants,
    config: BootstrapAscendingConfig,
}

impl PeerScoring {
    pub fn new(network_constants: NetworkConstants, config: BootstrapAscendingConfig) -> Self {
        Self {
            scoring: Scoring::default(),
            network_constants,
            config,
        }
    }

    fn received_message(&mut self, channel: &Arc<ChannelEnum>) {
        self.scoring.modify(channel.channel_id(), |i| {
            if i.outstanding > 1 {
                i.outstanding -= 1;
                i.response_count_total += 1;
            }
        })
    }

    fn channel(&mut self) -> Option<Arc<ChannelEnum>> {
        if let Some(channel) = self.get_next_channel() {
            self.scoring.modify(channel.channel_id(), |i| {
                i.outstanding += 1;
                i.request_count_total += 1;
            });
            Some(channel)
        } else {
            None
        }
    }

    fn get_next_channel(&self) -> Option<Arc<ChannelEnum>> {
        self.scoring.iter_by_outstanding().find_map(|score| {
            if let Some(channel) = score.channel.upgrade() {
                if !channel.max(TrafficType::Generic)
                    && score.outstanding < self.config.requests_limit
                {
                    return Some(channel);
                }
            }
            None
        })
    }

    fn len(&self) -> usize {
        self.scoring.len()
    }

    fn timeout(&mut self) {
        self.scoring.retain(|i| i.is_alive());
        self.scoring.modify_all(|i| i.decay());
    }

    fn sync(&mut self, channels: &[Arc<ChannelEnum>]) {
        for channel in channels {
            if channel.network_version() >= self.network_constants.bootstrap_protocol_version_min {
                if !self.scoring.contains(channel.channel_id()) {
                    if !channel.max(TrafficType::Bootstrap) {
                        self.scoring.insert(PeerScore::new(channel));
                    }
                }
            }
        }
    }
}

struct PeerScore {
    channel_id: usize,
    channel: Weak<ChannelEnum>,
    /// Number of outstanding requests to a peer
    outstanding: usize,
    request_count_total: usize,
    response_count_total: usize,
}

impl PeerScore {
    fn new(channel: &Arc<ChannelEnum>) -> Self {
        Self {
            channel_id: channel.channel_id(),
            channel: Arc::downgrade(channel),
            outstanding: 1,
            request_count_total: 1,
            response_count_total: 0,
        }
    }

    fn is_alive(&self) -> bool {
        self.channel
            .upgrade()
            .map(|i| i.is_alive())
            .unwrap_or(false)
    }

    fn decay(&mut self) {
        if self.outstanding > 0 {
            self.outstanding -= 1;
        }
    }
}

#[derive(Default)]
struct Scoring {
    by_channel: HashMap<usize, PeerScore>,
    by_outstanding: BTreeMap<usize, Vec<usize>>,
}

impl Scoring {
    fn len(&self) -> usize {
        self.by_channel.len()
    }

    fn get(&self, channel_id: usize) -> Option<&PeerScore> {
        self.by_channel.get(&channel_id)
    }

    fn contains(&self, channel_id: usize) -> bool {
        self.by_channel.contains_key(&channel_id)
    }

    fn insert(&mut self, score: PeerScore) -> Option<PeerScore> {
        let outstanding = score.outstanding;
        let channel_id = score.channel_id;

        let old = self.by_channel.insert(score.channel_id, score);

        if let Some(old) = &old {
            self.remove_outstanding(old.channel_id, old.outstanding);
        }

        self.insert_outstanding(channel_id, outstanding);
        old
    }

    fn modify(&mut self, channel_id: usize, mut f: impl FnMut(&mut PeerScore)) {
        if let Some(scoring) = self.by_channel.get_mut(&channel_id) {
            let old_outstanding = scoring.outstanding;
            f(scoring);
            let new_outstanding = scoring.outstanding;
            if new_outstanding != old_outstanding {
                self.remove_outstanding(channel_id, old_outstanding);
                self.insert_outstanding(channel_id, new_outstanding);
            }
        }
    }

    fn modify_all(&mut self, mut f: impl FnMut(&mut PeerScore)) {
        let channel_ids: Vec<usize> = self.by_channel.keys().cloned().collect();
        for id in channel_ids {
            self.modify(id, &mut f);
        }
    }

    fn retain(&mut self, mut f: impl FnMut(&PeerScore) -> bool) {
        let to_delete = self
            .by_channel
            .values()
            .filter(|i| f(i))
            .map(|i| i.channel_id)
            .collect::<Vec<_>>();

        for channel_id in to_delete {
            self.remove(channel_id);
        }
    }

    fn remove(&mut self, channel_id: usize) {
        if let Some(scoring) = self.by_channel.remove(&channel_id) {
            self.remove_outstanding(channel_id, scoring.outstanding);
        }
    }

    fn insert_outstanding(&mut self, channel_id: usize, outstanding: usize) {
        self.by_outstanding
            .entry(outstanding)
            .or_default()
            .push(channel_id);
    }

    fn remove_outstanding(&mut self, channel_id: usize, outstanding: usize) {
        let channel_ids = self.by_outstanding.get_mut(&outstanding).unwrap();
        if channel_ids.len() > 1 {
            channel_ids.retain(|i| *i != channel_id);
        } else {
            self.by_outstanding.remove(&outstanding);
        }
    }

    fn iter_by_outstanding(&self) -> impl Iterator<Item = &PeerScore> {
        self.by_outstanding
            .values()
            .flatten()
            .map(|id| self.by_channel.get(id).unwrap())
    }
}
