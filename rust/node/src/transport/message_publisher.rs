use super::{Channel, ChannelId, DropPolicy, Network, TrafficType};
use crate::{
    representatives::OnlineReps,
    stats::{Direction, StatType, Stats},
};
use rsnano_messages::{Message, MessageSerializer, ProtocolInfo};
use std::sync::{Arc, Mutex};
use tracing::trace;

/// Publishes messages to peered nodes
#[derive(Clone)]
pub struct MessagePublisher {
    online_reps: Arc<Mutex<OnlineReps>>,
    network: Arc<Network>,
    stats: Arc<Stats>,
    message_serializer: MessageSerializer,
}

impl MessagePublisher {
    pub(crate) fn new(
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
        }
    }

    pub fn try_send(
        &mut self,
        channel_id: ChannelId,
        message: &Message,
        drop_policy: DropPolicy,
        traffic_type: TrafficType,
    ) -> bool {
        let buffer = self.message_serializer.serialize(message);
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

        for peer in self.list_no_pr(self.network.fanout(scale)) {
            self.try_send(peer.channel_id(), &message, drop_policy, traffic_type);
        }
    }

    fn list_no_pr(&self, count: usize) -> Vec<Arc<Channel>> {
        let mut channels = self.network.random_list_realtime(usize::MAX, 0);
        {
            let reps = self.online_reps.lock().unwrap();
            channels.retain(|c| !reps.is_pr(c.channel_id()));
        }
        channels.truncate(count);
        channels
    }
}
