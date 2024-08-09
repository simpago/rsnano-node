use super::VoteProcessorQueue;
use crate::{
    representatives::OnlineReps,
    transport::{BufferDropPolicy, ChannelId, Network, TrafficType},
};
use rsnano_core::{Vote, VoteSource};
use rsnano_messages::{ConfirmAck, Message};
use std::{
    ops::Deref,
    sync::{Arc, Mutex},
};

/// Broadcast a vote to PRs and some non-PRs
pub struct VoteBroadcaster {
    online_reps: Arc<Mutex<OnlineReps>>,
    network: Arc<Network>,
    vote_processor_queue: Arc<VoteProcessorQueue>,
}

impl VoteBroadcaster {
    pub fn new(
        online_reps: Arc<Mutex<OnlineReps>>,
        network: Arc<Network>,
        vote_processor_queue: Arc<VoteProcessorQueue>,
    ) -> Self {
        Self {
            online_reps,
            network,
            vote_processor_queue,
        }
    }

    /// Broadcast vote to PRs and some non-PRs
    pub fn broadcast(&self, vote: Arc<Vote>) {
        self.flood_vote_pr(vote.deref().clone());

        let ack = Message::ConfirmAck(ConfirmAck::new_with_own_vote(vote.deref().clone()));
        self.network.flood_message(&ack, 2.0);

        self.vote_processor_queue
            .vote(vote, ChannelId::LOOPBACK, VoteSource::Live);
    }

    fn flood_vote_pr(&self, vote: Vote) {
        let message = Message::ConfirmAck(ConfirmAck::new_with_own_vote(vote));
        for rep in self.online_reps.lock().unwrap().peered_reps() {
            self.network.try_send(
                rep.channel_id,
                &message,
                BufferDropPolicy::NoLimiterDrop,
                TrafficType::Generic,
            )
        }
    }
}
