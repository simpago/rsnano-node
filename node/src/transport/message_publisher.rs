use crate::stats::{Direction, StatType, Stats};
use rsnano_messages::{Message, MessageSerializer, ProtocolInfo};
use rsnano_network::{ChannelId, DropPolicy, Network, TrafficType};
use std::sync::Arc;
use tracing::trace;

pub type MessageCallback = Arc<dyn Fn(ChannelId, &Message) + Send + Sync>;

/// Publishes messages to peered nodes
#[derive(Clone)]
pub struct MessagePublisher {
    network: Arc<Network>,
    stats: Arc<Stats>,
    message_serializer: MessageSerializer,
    published_callback: Option<MessageCallback>,
}

impl MessagePublisher {
    pub fn new(network: Arc<Network>, stats: Arc<Stats>, protocol_info: ProtocolInfo) -> Self {
        Self {
            network,
            stats,
            message_serializer: MessageSerializer::new(protocol_info),
            published_callback: None,
        }
    }

    pub fn new_with_buffer_size(
        network: Arc<Network>,
        stats: Arc<Stats>,
        protocol_info: ProtocolInfo,
        buffer_size: usize,
    ) -> Self {
        Self {
            network,
            stats,
            message_serializer: MessageSerializer::new_with_buffer_size(protocol_info, buffer_size),
            published_callback: None,
        }
    }

    pub fn set_published_callback(&mut self, callback: MessageCallback) {
        self.published_callback = Some(callback);
    }

    pub(crate) fn new_null(handle: tokio::runtime::Handle) -> Self {
        Self::new(
            Arc::new(Network::new_null(handle)),
            Arc::new(Stats::default()),
            Default::default(),
        )
    }

    pub fn try_send(
        &mut self,
        channel_id: ChannelId,
        message: &Message,
        drop_policy: DropPolicy,
        traffic_type: TrafficType,
    ) -> bool {
        let buffer = self.message_serializer.serialize(message);
        let sent = try_send_serialized_message(
            &self.network,
            &self.stats,
            channel_id,
            buffer,
            message,
            drop_policy,
            traffic_type,
        );

        if let Some(callback) = &self.published_callback {
            callback(channel_id, message);
        }

        sent
    }

    pub async fn send(
        &mut self,
        channel_id: ChannelId,
        message: &Message,
        traffic_type: TrafficType,
    ) -> anyhow::Result<()> {
        let buffer = self.message_serializer.serialize(message);
        self.network
            .send_buffer(channel_id, &buffer, traffic_type)
            .await?;
        self.stats
            .inc_dir_aggregate(StatType::Message, message.into(), Direction::Out);
        trace!(%channel_id, message = ?message, "Message sent");

        if let Some(callback) = &self.published_callback {
            callback(channel_id, message);
        }

        Ok(())
    }

    pub fn get_serializer(&self) -> MessageSerializer {
        self.message_serializer.clone()
    }

    pub fn try_send_serialized_message(
        &self,
        channel_id: ChannelId,
        buffer: &[u8],
        message: &Message,
        drop_policy: DropPolicy,
        traffic_type: TrafficType,
    ) -> bool {
        let sent = self
            .network
            .try_send_buffer(channel_id, buffer, drop_policy, traffic_type);

        if sent {
            self.stats
                .inc_dir_aggregate(StatType::Message, message.into(), Direction::Out);
            trace!(%channel_id, message = ?message, "Message sent");
        } else {
            let detail_type = message.into();
            self.stats
                .inc_dir_aggregate(StatType::Drop, detail_type, Direction::Out);
            trace!(%channel_id, message = ?message, "Message dropped");
        }

        sent
    }
}

pub(crate) fn try_send_serialized_message(
    network: &Network,
    stats: &Stats,
    channel_id: ChannelId,
    buffer: &[u8],
    message: &Message,
    drop_policy: DropPolicy,
    traffic_type: TrafficType,
) -> bool {
    let sent = network.try_send_buffer(channel_id, buffer, drop_policy, traffic_type);

    if sent {
        stats.inc_dir_aggregate(StatType::Message, message.into(), Direction::Out);
        trace!(%channel_id, message = ?message, "Message sent");
    } else {
        let detail_type = message.into();
        stats.inc_dir_aggregate(StatType::Drop, detail_type, Direction::Out);
        trace!(%channel_id, message = ?message, "Message dropped");
    }

    sent
}
