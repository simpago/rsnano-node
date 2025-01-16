use super::{vote_generator::VoteGenerator, LocalVoteHistory};
use crate::{
    config::NodeConfig, consensus::VoteBroadcaster, stats::Stats, transport::MessageSender,
    wallets::Wallets, NetworkParams,
};
use rsnano_core::{utils::ContainerInfo, BlockHash, Root, SavedBlock};
use rsnano_ledger::Ledger;
use rsnano_network::ChannelId;
use std::{sync::Arc, time::Duration};

pub struct VoteGenerators {
    non_final_vote_generator: VoteGenerator,
    final_vote_generator: VoteGenerator,
}

impl VoteGenerators {
    pub(crate) fn new(
        ledger: Arc<Ledger>,
        wallets: Arc<Wallets>,
        history: Arc<LocalVoteHistory>,
        stats: Arc<Stats>,
        config: &NodeConfig,
        network_params: &NetworkParams,
        vote_broadcaster: Arc<VoteBroadcaster>,
        message_publisher: MessageSender,
    ) -> Self {
        let non_final_vote_generator = VoteGenerator::new(
            ledger.clone(),
            wallets.clone(),
            history.clone(),
            false, //none-final
            stats.clone(),
            message_publisher.clone(),
            Duration::from_secs(network_params.voting.delay_s as u64),
            Duration::from_millis(config.vote_generator_delay_ms as u64),
            vote_broadcaster.clone(),
        );

        let final_vote_generator = VoteGenerator::new(
            ledger,
            wallets,
            history,
            true, //final
            stats,
            message_publisher.clone(),
            Duration::from_secs(network_params.voting.delay_s as u64),
            Duration::from_millis(config.vote_generator_delay_ms as u64),
            vote_broadcaster,
        );

        Self {
            non_final_vote_generator,
            final_vote_generator,
        }
    }

    pub fn start(&self) {
        self.non_final_vote_generator.start();
        self.final_vote_generator.start();
    }

    pub fn stop(&self) {
        self.non_final_vote_generator.stop();
        self.final_vote_generator.stop();
    }

    pub(crate) fn generate_final_vote(&self, root: &Root, hash: &BlockHash) {
        self.final_vote_generator.add(root, hash);
    }

    pub(crate) fn generate_final_votes(
        &self,
        blocks: &[SavedBlock],
        channel_id: ChannelId,
    ) -> usize {
        self.final_vote_generator.generate(blocks, channel_id)
    }

    pub fn generate_non_final_vote(&self, root: &Root, hash: &BlockHash) {
        self.non_final_vote_generator.add(root, hash);
    }

    pub fn generate_non_final_votes(&self, blocks: &[SavedBlock], channel_id: ChannelId) -> usize {
        self.non_final_vote_generator.generate(blocks, channel_id)
    }

    pub(crate) fn container_info(&self) -> ContainerInfo {
        ContainerInfo::builder()
            .node("non_final", self.non_final_vote_generator.container_info())
            .node("final", self.final_vote_generator.container_info())
            .finish()
    }
}
