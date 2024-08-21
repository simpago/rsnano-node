use super::{Election, ElectionData};
use crate::{
    representatives::PeeredRep,
    transport::{ChannelId, DropPolicy, MessagePublisher, NetworkInfo, TrafficType},
    NetworkParams,
};
use rsnano_core::{BlockHash, Root};
use rsnano_messages::{ConfirmReq, Message, Publish};
use std::{
    cmp::max,
    collections::{HashMap, HashSet},
    ops::Deref,
    sync::{atomic::Ordering, MutexGuard, RwLock},
};

/// This struct accepts elections that need further votes before they can be confirmed and bundles them in to single confirm_req packets
pub struct ConfirmationSolicitor<'a> {
    network_info: &'a RwLock<NetworkInfo>,
    /// Global maximum amount of block broadcasts
    max_block_broadcasts: usize,
    /// Maximum amount of requests to be sent per election, bypassed if an existing vote is for a different hash
    max_election_requests: usize,
    /// Maximum amount of directed broadcasts to be sent per election
    max_election_broadcasts: usize,
    representative_requests: Vec<PeeredRep>,
    representative_broadcasts: Vec<PeeredRep>,
    requests: HashMap<ChannelId, Vec<(BlockHash, Root)>>,
    channels: HashSet<ChannelId>,
    prepared: bool,
    rebroadcasted: usize,
    message_publisher: MessagePublisher,
}

impl<'a> ConfirmationSolicitor<'a> {
    pub fn new(
        network_params: &NetworkParams,
        network_info: &'a RwLock<NetworkInfo>,
        message_publisher: MessagePublisher,
    ) -> Self {
        let max_election_broadcasts = max(network_info.read().unwrap().fanout(1.0) / 2, 1);
        Self {
            network_info,
            max_block_broadcasts: if network_params.network.is_dev_network() {
                4
            } else {
                30
            },
            max_election_requests: 50,
            max_election_broadcasts,
            prepared: false,
            representative_requests: Vec::new(),
            representative_broadcasts: Vec::new(),
            requests: HashMap::new(),
            channels: HashSet::new(),
            rebroadcasted: 0,
            message_publisher,
        }
    }

    /// Prepare object for batching election confirmation requests
    pub fn prepare(&mut self, representatives: &[PeeredRep]) {
        debug_assert!(!self.prepared);
        self.requests.clear();
        self.rebroadcasted = 0;
        self.representative_requests = representatives.to_vec();
        self.representative_broadcasts = representatives.to_vec();
        self.prepared = true;
    }

    /// Broadcast the winner of an election if the broadcast limit has not been reached. Returns false if the broadcast was performed
    pub fn broadcast(&mut self, guard: &MutexGuard<ElectionData>) -> Result<(), ()> {
        debug_assert!(self.prepared);
        self.rebroadcasted += 1;
        if self.rebroadcasted >= self.max_block_broadcasts {
            return Err(());
        }

        let winner_block = guard.status.winner.as_ref().unwrap();
        let hash = winner_block.hash();
        let winner = Message::Publish(Publish::new_forward(winner_block.deref().clone()));
        let mut count = 0;
        // Directed broadcasting to principal representatives
        for i in &self.representative_broadcasts {
            if count >= self.max_election_broadcasts {
                break;
            }
            let should_broadcast = if let Some(existing) = guard.last_votes.get(&i.account) {
                existing.hash != hash
            } else {
                count += 1;
                true
            };
            if should_broadcast {
                self.message_publisher.try_send(
                    i.channel_id,
                    &winner,
                    DropPolicy::CanDrop,
                    TrafficType::Generic,
                );
            }
        }
        // Random flood for block propagation
        self.message_publisher
            .flood(&winner, DropPolicy::CanDrop, 0.5);
        Ok(())
    }

    /// Add an election that needs to be confirmed. Returns false if successfully added
    pub fn add(&mut self, election: &Election, guard: &MutexGuard<ElectionData>) -> bool {
        debug_assert!(self.prepared);
        let mut error = true;
        let mut count = 0;
        let winner = guard.status.winner.as_ref().unwrap();
        let hash = winner.hash();
        let mut to_remove = Vec::new();
        for rep in &self.representative_requests {
            if count >= self.max_election_requests {
                break;
            }
            let mut full_queue = false;
            let existing = guard.last_votes.get(&rep.account);
            let exists = existing.is_some();
            let is_final = if let Some(existing) = existing {
                !election.is_quorum.load(Ordering::SeqCst) || existing.timestamp == u64::MAX
            } else {
                false
            };
            let different = if let Some(existing) = existing {
                existing.hash != hash
            } else {
                false
            };
            if !exists || !is_final || different {
                let request_queue = self.requests.entry(rep.channel_id).or_default();
                self.channels.insert(rep.channel_id);
                if !self
                    .network_info
                    .read()
                    .unwrap()
                    .is_queue_full(rep.channel_id, TrafficType::Generic)
                {
                    request_queue.push((winner.hash(), winner.root()));
                    if !different {
                        count += 1;
                    }
                    error = false;
                } else {
                    full_queue = true;
                }
            }
            if full_queue {
                to_remove.push(rep.account);
            }
        }

        if !to_remove.is_empty() {
            self.representative_requests
                .retain(|i| !to_remove.contains(&i.account));
        }

        error
    }

    /// Dispatch bundled requests to each channel
    pub fn flush(&mut self) {
        debug_assert!(self.prepared);
        for channel_id in &self.channels {
            let mut roots_hashes = Vec::new();
            if let Some(requests) = self.requests.get(channel_id) {
                for root_hash in requests {
                    roots_hashes.push(root_hash.clone());
                    if roots_hashes.len() == ConfirmReq::HASHES_MAX {
                        let req = Message::ConfirmReq(ConfirmReq::new(roots_hashes));
                        self.message_publisher.try_send(
                            *channel_id,
                            &req,
                            DropPolicy::CanDrop,
                            TrafficType::Generic,
                        );
                        roots_hashes = Vec::new();
                    }
                }
            }
            if !roots_hashes.is_empty() {
                let req = Message::ConfirmReq(ConfirmReq::new(roots_hashes));
                self.message_publisher.try_send(
                    *channel_id,
                    &req,
                    DropPolicy::CanDrop,
                    TrafficType::Generic,
                );
            }
        }
        self.prepared = false;
    }
}
