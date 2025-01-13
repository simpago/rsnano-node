use super::BootstrapConfig;
use rsnano_core::utils::ContainerInfo;
use rsnano_network::{Channel, ChannelId, TrafficType};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Weak},
};

/// Container for tracking and scoring peers with respect to bootstrapping
pub(crate) struct PeerScoring {
    scoring: OrderedScoring,
    config: BootstrapConfig,
    channels: Vec<Arc<Channel>>,
}

impl PeerScoring {
    pub fn new(config: BootstrapConfig) -> Self {
        Self {
            scoring: OrderedScoring::default(),
            config,
            channels: Vec::new(),
        }
    }

    pub fn received_message(&mut self, channel_id: ChannelId) {
        self.scoring.modify(channel_id, |i| {
            if i.outstanding > 1 {
                i.outstanding -= 1;
                i.response_count_total += 1;
            }
        });
    }

    pub fn channel(&mut self) -> Option<Arc<Channel>> {
        self.channels
            .iter()
            .find(|c| {
                if !c.is_queue_full(TrafficType::Bootstrap) {
                    if !Self::try_send_message(&mut self.scoring, c, &self.config) {
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
            .cloned()
    }

    fn try_send_message(
        scoring: &mut OrderedScoring,
        channel: &Arc<Channel>,
        config: &BootstrapConfig,
    ) -> bool {
        let mut result = false;
        let modified = scoring.modify(channel.channel_id(), |i| {
            if i.outstanding < config.channel_limit {
                i.outstanding += 1;
                i.request_count_total += 1;
            } else {
                result = true;
            }
        });
        if !modified {
            scoring.insert(PeerScore::new(channel));
        }
        result
    }

    pub fn len(&self) -> usize {
        self.scoring.len()
    }

    pub fn available(&self) -> usize {
        self.channels
            .iter()
            .filter(|c| !self.limit_exceeded(c))
            .count()
    }

    fn limit_exceeded(&self, channel: &Channel) -> bool {
        if let Some(existing) = self.scoring.get(channel.channel_id()) {
            existing.outstanding >= self.config.channel_limit
        } else {
            false
        }
    }

    pub fn timeout(&mut self) {
        self.scoring.retain(|i| i.is_alive());
        self.scoring.modify_all(|i| i.decay());
    }

    // Synchronize channels with the network, passed channels should be shuffled
    pub fn sync(&mut self, channels: Vec<Arc<Channel>>) {
        self.channels = channels;
    }

    pub fn container_info(&self) -> ContainerInfo {
        [
            ("scores", self.len(), 0),
            ("available", self.available(), 0),
            ("channels", self.channels.len(), 0),
        ]
        .into()
    }
}

struct PeerScore {
    channel_id: ChannelId,
    channel: Weak<Channel>,
    /// Number of outstanding requests to a peer
    outstanding: usize,
    request_count_total: usize,
    response_count_total: usize,
}

impl PeerScore {
    fn new(channel: &Arc<Channel>) -> Self {
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
struct OrderedScoring {
    by_channel: HashMap<ChannelId, PeerScore>,
    by_outstanding: BTreeMap<usize, Vec<ChannelId>>,
}

impl OrderedScoring {
    fn len(&self) -> usize {
        self.by_channel.len()
    }

    #[allow(dead_code)]
    fn get(&self, channel_id: ChannelId) -> Option<&PeerScore> {
        self.by_channel.get(&channel_id)
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

    fn modify(&mut self, channel_id: ChannelId, mut f: impl FnMut(&mut PeerScore)) -> bool {
        if let Some(scoring) = self.by_channel.get_mut(&channel_id) {
            let old_outstanding = scoring.outstanding;
            f(scoring);
            let new_outstanding = scoring.outstanding;
            if new_outstanding != old_outstanding {
                self.remove_outstanding(channel_id, old_outstanding);
                self.insert_outstanding(channel_id, new_outstanding);
            }
            true
        } else {
            false
        }
    }

    fn modify_all(&mut self, mut f: impl FnMut(&mut PeerScore)) {
        let channel_ids: Vec<ChannelId> = self.by_channel.keys().cloned().collect();
        for id in channel_ids {
            self.modify(id, &mut f);
        }
    }

    fn retain(&mut self, mut f: impl FnMut(&PeerScore) -> bool) {
        let to_delete = self
            .by_channel
            .values()
            .filter(|i| !f(i))
            .map(|i| i.channel_id)
            .collect::<Vec<_>>();

        for channel_id in to_delete {
            self.remove(channel_id);
        }
    }

    fn remove(&mut self, channel_id: ChannelId) {
        if let Some(scoring) = self.by_channel.remove(&channel_id) {
            self.remove_outstanding(channel_id, scoring.outstanding);
        }
    }

    fn insert_outstanding(&mut self, channel_id: ChannelId, outstanding: usize) {
        self.by_outstanding
            .entry(outstanding)
            .or_default()
            .push(channel_id);
    }

    fn remove_outstanding(&mut self, channel_id: ChannelId, outstanding: usize) {
        let channel_ids = self.by_outstanding.get_mut(&outstanding).unwrap();
        if channel_ids.len() > 1 {
            channel_ids.retain(|i| *i != channel_id);
        } else {
            self.by_outstanding.remove(&outstanding);
        }
    }
}
