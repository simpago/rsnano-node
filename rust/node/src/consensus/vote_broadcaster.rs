use super::VoteProcessorQueue;
use crate::{
    representatives::OnlineReps,
    transport::{ChannelId, DropPolicy, MessagePublisher, Network, TrafficType},
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
    message_publisher: Mutex<MessagePublisher>,
}

impl VoteBroadcaster {
    pub fn new(
        online_reps: Arc<Mutex<OnlineReps>>,
        network: Arc<Network>,
        vote_processor_queue: Arc<VoteProcessorQueue>,
        message_publisher: MessagePublisher,
    ) -> Self {
        Self {
            online_reps,
            network,
            vote_processor_queue,
            message_publisher: Mutex::new(message_publisher),
        }
    }

    /// Broadcast vote to PRs and some non-PRs
    pub fn broadcast(&self, vote: Arc<Vote>) {
        let ack = Message::ConfirmAck(ConfirmAck::new_with_own_vote(vote.deref().clone()));

        self.message_publisher
            .lock()
            .unwrap()
            .flood_prs_and_some_non_prs(&ack, DropPolicy::ShouldNotDrop, TrafficType::Generic, 2.0);

        self.vote_processor_queue
            .vote(vote, ChannelId::LOOPBACK, VoteSource::Live);
    }
}
