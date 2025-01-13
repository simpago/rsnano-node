use super::{try_send_serialized_message, MessagePublisher};
use crate::{representatives::OnlineReps, stats::Stats};
use rsnano_messages::{Message, MessageSerializer};
use rsnano_network::{ChannelInfo, DropPolicy, Network, TrafficType};
use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

/// Floods messages to PRs and non PRs
#[derive(Clone)]
pub struct MessageFlooder {
    online_reps: Arc<Mutex<OnlineReps>>,
    network: Arc<Network>,
    stats: Arc<Stats>,
    message_serializer: MessageSerializer,
    publisher: MessagePublisher,
}

impl MessageFlooder {
    pub fn new(
        online_reps: Arc<Mutex<OnlineReps>>,
        network: Arc<Network>,
        stats: Arc<Stats>,
        publisher: MessagePublisher,
    ) -> Self {
        Self {
            online_reps,
            network,
            stats,
            message_serializer: publisher.get_serializer(),
            publisher,
        }
    }

    pub(crate) fn new_null(handle: tokio::runtime::Handle) -> Self {
        Self::new(
            Arc::new(Mutex::new(OnlineReps::default())),
            Arc::new(Network::new_null(handle.clone())),
            Arc::new(Stats::default()),
            MessagePublisher::new_null(),
        )
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
            self.publisher
                .try_send(rep.channel_id, &message, drop_policy, traffic_type);
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
            self.publisher
                .try_send(peer.channel_id(), &message, drop_policy, traffic_type);
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

        let network_info = self.network.info.read().unwrap();
        for channel in channels {
            try_send_serialized_message(
                &network_info,
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

impl Deref for MessageFlooder {
    type Target = MessagePublisher;

    fn deref(&self) -> &Self::Target {
        &self.publisher
    }
}

impl DerefMut for MessageFlooder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.publisher
    }
}
