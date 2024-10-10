use crate::{
    representatives::OnlineReps,
    stats::{Direction, StatType, Stats},
};
use rsnano_messages::{Message, MessageSerializer, ProtocolInfo};
use rsnano_network::{ChannelId, ChannelInfo, DropPolicy, Network, TrafficType};
use std::sync::{Arc, Mutex};
use tracing::trace;

pub type MessageCallback = Arc<dyn Fn(ChannelId, &Message) + Send + Sync>;

/// Publishes messages to peered nodes
#[derive(Clone)]
pub struct MessagePublisher {
    online_reps: Arc<Mutex<OnlineReps>>,
    network: Arc<Network>,
    stats: Arc<Stats>,
    message_serializer: MessageSerializer,
    published_callback: Option<MessageCallback>,
}

impl MessagePublisher {
    pub fn new(
        online_reps: Arc<Mutex<OnlineReps>>,
        network: Arc<Network>,
        stats: Arc<Stats>,
        protocol_info: ProtocolInfo,
    ) -> Self {
        Self {
            online_reps,
            network,
            stats,
            message_serializer: MessageSerializer::new(protocol_info),
            published_callback: None,
        }
    }

    pub fn new_with_buffer_size(
        online_reps: Arc<Mutex<OnlineReps>>,
        network: Arc<Network>,
        stats: Arc<Stats>,
        protocol_info: ProtocolInfo,
        buffer_size: usize,
    ) -> Self {
        Self {
            online_reps,
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
            Arc::new(Mutex::new(OnlineReps::default())),
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

    pub(crate) fn flood_prs_and_some_non_prs(
        &mut self,
        message: &Message,
        drop_policy: DropPolicy,
        traffic_type: TrafficType,
        scale: f32,
    ) {
        let peered_prs = self.online_reps.lock().unwrap().peered_principal_reps();
        for rep in peered_prs {
            self.try_send(rep.channel_id, &message, drop_policy, traffic_type);
        }

        let mut channels;
        let fanout;
        {
            let network = self.network.info.read().unwrap();
            fanout = network.fanout(scale);
            channels = network.random_list_realtime(usize::MAX, 0)
        }

        self.remove_no_pr(&mut channels, fanout);
        for peer in channels {
            self.try_send(peer.channel_id(), &message, drop_policy, traffic_type);
        }
    }

    fn remove_no_pr(&self, channels: &mut Vec<Arc<ChannelInfo>>, count: usize) {
        {
            let reps = self.online_reps.lock().unwrap();
            channels.retain(|c| !reps.is_pr(c.channel_id()));
        }
        channels.truncate(count);
    }

    pub fn flood(&mut self, message: &Message, drop_policy: DropPolicy, scale: f32) {
        let buffer = self.message_serializer.serialize(message);
        let channels = self
            .network
            .info
            .read()
            .unwrap()
            .random_fanout_realtime(scale);

        for channel in channels {
            try_send_serialized_message(
                &self.network,
                &self.stats,
                channel.channel_id(),
                buffer,
                message,
                drop_policy,
                TrafficType::Generic,
            );
        }
    }
}

fn try_send_serialized_message(
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
