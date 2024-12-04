use super::{FairQueue, MessageCallback};
use crate::stats::{DetailType, StatType, Stats};
use rsnano_core::utils::ContainerInfo;
use rsnano_messages::Message;
use rsnano_network::{ChannelId, ChannelInfo, DeadChannelCleanupStep};
use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex},
};

pub struct InboundMessageQueue {
    state: Mutex<State>,
    condition: Condvar,
    stats: Arc<Stats>,
    inbound_callback: Option<MessageCallback>,
    inbound_dropped_callback: Option<MessageCallback>,
}

impl InboundMessageQueue {
    pub fn new(max_queue: usize, stats: Arc<Stats>) -> Self {
        Self {
            state: Mutex::new(State {
                queue: FairQueue::new(Box::new(move |_| max_queue), Box::new(|_| 1)),
                stopped: false,
            }),
            condition: Condvar::new(),
            stats,
            inbound_callback: None,
            inbound_dropped_callback: None,
        }
    }

    pub fn set_inbound_callback(&mut self, callback: MessageCallback) {
        self.inbound_callback = Some(callback);
    }

    pub fn set_inbound_dropped_callback(&mut self, callback: MessageCallback) {
        self.inbound_dropped_callback = Some(callback);
    }

    pub fn put(&self, message: Message, channel: Arc<ChannelInfo>) -> bool {
        let message_type = message.message_type();
        let added = self
            .state
            .lock()
            .unwrap()
            .queue
            .push(channel.channel_id(), (message.clone(), channel.clone()));

        if added {
            self.stats
                .inc(StatType::MessageProcessor, DetailType::Process);
            self.stats
                .inc(StatType::MessageProcessorType, message_type.into());

            self.condition.notify_all();
            if let Some(cb) = &self.inbound_callback {
                cb(channel.channel_id(), &message);
            }
        } else {
            self.stats
                .inc(StatType::MessageProcessor, DetailType::Overfill);
            self.stats
                .inc(StatType::MessageProcessorOverfill, message_type.into());
            if let Some(cb) = &self.inbound_dropped_callback {
                cb(channel.channel_id(), &message);
            }
        }

        added
    }

    pub(crate) fn next_batch(
        &self,
        max_batch_size: usize,
    ) -> VecDeque<(ChannelId, (Message, Arc<ChannelInfo>))> {
        self.state.lock().unwrap().queue.next_batch(max_batch_size)
    }

    pub fn wait_for_messages(&self) {
        let state = self.state.lock().unwrap();
        if !state.queue.is_empty() {
            return;
        }
        drop(
            self.condition
                .wait_while(state, |s| !s.stopped && s.queue.is_empty()),
        )
    }

    pub fn size(&self) -> usize {
        self.state.lock().unwrap().queue.len()
    }

    /// Stop container and notify waiting threads
    pub fn stop(&self) {
        {
            let mut lock = self.state.lock().unwrap();
            lock.stopped = true;
        }
        self.condition.notify_all();
    }

    pub fn container_info(&self) -> ContainerInfo {
        let guard = self.state.lock().unwrap();
        ContainerInfo::builder()
            .node("queue", guard.queue.container_info())
            .finish()
    }
}

impl Default for InboundMessageQueue {
    fn default() -> Self {
        Self::new(64, Arc::new(Stats::default()))
    }
}

pub struct InboundMessageQueueCleanup(Arc<InboundMessageQueue>);

impl InboundMessageQueueCleanup {
    pub fn new(queue: Arc<InboundMessageQueue>) -> Self {
        Self(queue)
    }
}

impl DeadChannelCleanupStep for InboundMessageQueueCleanup {
    fn clean_up_dead_channels(&self, dead_channel_ids: &[ChannelId]) {
        let mut guard = self.0.state.lock().unwrap();
        for channel_id in dead_channel_ids {
            guard.queue.remove(channel_id);
        }
    }
}

struct State {
    queue: FairQueue<ChannelId, (Message, Arc<ChannelInfo>)>,
    stopped: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsnano_messages::Message;

    #[test]
    fn put_and_get_one_message() {
        let manager = InboundMessageQueue::new(1, Arc::new(Stats::default()));
        assert_eq!(manager.size(), 0);
        manager.put(
            Message::BulkPush,
            Arc::new(ChannelInfo::new_test_instance()),
        );
        assert_eq!(manager.size(), 1);
        assert_eq!(manager.next_batch(1000).len(), 1);
        assert_eq!(manager.size(), 0);
    }
}
