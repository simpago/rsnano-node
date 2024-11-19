use super::NetworkFilter;
use crate::{
    block_processing::{BlockProcessor, BlockSource},
    bootstrap::{BootstrapAscending, BootstrapServer},
    config::NodeConfig,
    consensus::{RequestAggregator, VoteProcessorQueue},
    stats::{DetailType, Direction, StatType, Stats},
    wallets::Wallets,
    Telemetry,
};
use rsnano_core::VoteSource;
use rsnano_messages::Message;
use rsnano_network::{ChannelInfo, NetworkInfo};
use std::{
    net::SocketAddrV6,
    sync::{Arc, RwLock},
};
use tracing::trace;

/// Handle realtime messages (as opposed to bootstrap messages)
pub struct RealtimeMessageHandler {
    stats: Arc<Stats>,
    publish_filter: Arc<NetworkFilter>,
    network_info: Arc<RwLock<NetworkInfo>>,
    block_processor: Arc<BlockProcessor>,
    config: NodeConfig,
    wallets: Arc<Wallets>,
    request_aggregator: Arc<RequestAggregator>,
    vote_processor_queue: Arc<VoteProcessorQueue>,
    telemetry: Arc<Telemetry>,
    bootstrap_server: Arc<BootstrapServer>,
    ascend_boot: Arc<BootstrapAscending>,
}

impl RealtimeMessageHandler {
    pub(crate) fn new(
        stats: Arc<Stats>,
        network_info: Arc<RwLock<NetworkInfo>>,
        publish_filter: Arc<NetworkFilter>,
        block_processor: Arc<BlockProcessor>,
        config: NodeConfig,
        wallets: Arc<Wallets>,
        request_aggregator: Arc<RequestAggregator>,
        vote_processor_queue: Arc<VoteProcessorQueue>,
        telemetry: Arc<Telemetry>,
        bootstrap_server: Arc<BootstrapServer>,
        ascend_boot: Arc<BootstrapAscending>,
    ) -> Self {
        Self {
            stats,
            network_info,
            publish_filter,
            block_processor,
            config,
            wallets,
            request_aggregator,
            vote_processor_queue,
            telemetry,
            bootstrap_server,
            ascend_boot,
        }
    }

    pub fn process(&self, message: Message, channel: &Arc<ChannelInfo>) {
        self.stats.inc_dir(
            StatType::Message,
            message.message_type().into(),
            Direction::In,
        );
        trace!(?message, "network processed");

        match message {
            Message::Keepalive(keepalive) => {
                // Check for special node port data
                let peer0 = keepalive.peers[0];
                // The first entry is used to inform us of the peering address of the sending node
                if peer0.ip().is_unspecified() && peer0.port() != 0 {
                    let peering_addr =
                        SocketAddrV6::new(*channel.peer_addr().ip(), peer0.port(), 0, 0);

                    // Remember this for future forwarding to other peers
                    self.network_info
                        .read()
                        .unwrap()
                        .set_peering_addr(channel.channel_id(), peering_addr);
                }
            }
            Message::Publish(publish) => {
                // Put blocks that are being initally broadcasted in a separate queue, so that they won't have to compete with rebroadcasted blocks
                // Both queues have the same priority and size, so the potential for exploiting this is limited
                let source = if publish.is_originator {
                    BlockSource::LiveOriginator
                } else {
                    BlockSource::Live
                };
                let added =
                    self.block_processor
                        .add(Arc::new(publish.block), source, channel.channel_id());
                if !added {
                    self.publish_filter.clear(publish.digest);
                    self.stats
                        .inc_dir(StatType::Drop, DetailType::Publish, Direction::In);
                }
            }
            Message::ConfirmReq(req) => {
                // Don't load nodes with disabled voting
                // TODO: This check should be cached somewhere
                if self.config.enable_voting && self.wallets.voting_reps_count() > 0 {
                    self.request_aggregator
                        .request(req.roots_hashes, channel.channel_id());
                }
            }
            Message::ConfirmAck(ack) => {
                if !ack.vote().voting_account.is_zero() {
                    let source = match ack.is_rebroadcasted() {
                        true => VoteSource::Rebroadcast,
                        false => VoteSource::Live,
                    };
                    self.vote_processor_queue.vote(
                        Arc::new(ack.vote().clone()),
                        channel.channel_id(),
                        source,
                    );
                }
            }
            Message::NodeIdHandshake(_) => {
                self.stats.inc_dir(
                    StatType::Message,
                    DetailType::NodeIdHandshake,
                    Direction::In,
                );
            }
            Message::TelemetryReq => {
                // Ignore telemetry requests as telemetry is being periodically broadcasted since V25+
            }
            Message::TelemetryAck(ack) => self.telemetry.process(&ack, channel),
            Message::AscPullReq(req) => {
                self.bootstrap_server.request(req, channel.clone());
            }
            Message::AscPullAck(ack) => self.ascend_boot.process(&ack, channel.channel_id()),
            Message::FrontierReq(_)
            | Message::BulkPush
            | Message::BulkPull(_)
            | Message::BulkPullAccount(_) => unreachable!(),
        }
    }
}