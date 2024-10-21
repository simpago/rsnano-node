use chrono::{DateTime, TimeZone, Utc};
use rsnano_messages::{Message, MessageType};
use rsnano_network::{ChannelDirection, ChannelId};

#[derive(Clone)]
pub(crate) struct RecordedMessage {
    pub channel_id: ChannelId,
    pub message: Message,
    pub direction: ChannelDirection,
    pub date: DateTime<Utc>,
}

impl RecordedMessage {
    #[allow(dead_code)]
    pub fn new_test_instance() -> Self {
        Self {
            channel_id: 42.into(),
            message: Message::BulkPush,
            direction: ChannelDirection::Outbound,
            date: Utc.with_ymd_and_hms(2024, 10, 18, 12, 59, 0).unwrap(),
        }
    }
}

#[derive(Default, PartialEq, Eq, Debug)]
pub(crate) struct MessageFilter {
    channel_id: Option<ChannelId>,
    types: Vec<MessageType>,
}

impl MessageFilter {
    pub fn all() -> Self {
        Default::default()
    }

    pub fn channel(channel_id: ChannelId) -> Self {
        Self {
            channel_id: Some(channel_id),
            types: Vec::new(),
        }
    }

    pub fn include(&self, message: &RecordedMessage) -> bool {
        self.include_channel(message) && self.include_message_type(message)
    }

    pub fn with_types(&self, types: Vec<MessageType>) -> Self {
        Self {
            channel_id: self.channel_id.clone(),
            types,
        }
    }

    pub fn with_channel(&self, channel_id: Option<ChannelId>) -> Self {
        Self {
            channel_id,
            types: self.types.clone(),
        }
    }

    fn include_channel(&self, message: &RecordedMessage) -> bool {
        match self.channel_id {
            Some(id) => message.channel_id == id,
            None => true,
        }
    }

    fn include_message_type(&self, message: &RecordedMessage) -> bool {
        if self.types.is_empty() {
            return true;
        }

        for msg_type in &self.types {
            if message.message.message_type() == *msg_type {
                return true;
            }
        }

        false
    }
}

#[derive(Default)]
pub(crate) struct MessageCollection {
    all_messages: Vec<RecordedMessage>,
    filtered: Vec<RecordedMessage>,
    filter: MessageFilter,
}

impl MessageCollection {
    pub fn get(&self, index: usize) -> Option<RecordedMessage> {
        self.filtered.get(index).cloned()
    }

    pub fn len(&self) -> usize {
        self.filtered.len()
    }

    pub fn add(&mut self, message: RecordedMessage) {
        if self.filter.include(&message) {
            self.filtered.push(message.clone());
        }
        self.all_messages.push(message);
    }

    pub fn clear(&mut self) {
        self.all_messages.clear();
        self.filtered.clear();
    }

    pub fn current_filter(&self) -> &MessageFilter {
        &self.filter
    }

    pub fn filter_channel(&mut self, channel_id: Option<ChannelId>) {
        self.set_filter(self.filter.with_channel(channel_id))
    }

    pub fn filter_message_types(&mut self, types: impl IntoIterator<Item = MessageType>) {
        let types = types.into_iter().collect();
        self.set_filter(self.filter.with_types(types));
    }

    fn set_filter(&mut self, filter: MessageFilter) {
        self.filter = filter;
        self.filtered = self
            .all_messages
            .iter()
            .filter(|m| self.filter.include(m))
            .cloned()
            .collect();
    }
}

pub(crate) const AVALABLE_FILTER_TYPES: [MessageType; 12] = [
    MessageType::Keepalive,
    MessageType::Publish,
    MessageType::ConfirmReq,
    MessageType::ConfirmAck,
    MessageType::BulkPull,
    MessageType::BulkPush,
    MessageType::FrontierReq,
    MessageType::BulkPullAccount,
    MessageType::TelemetryReq,
    MessageType::TelemetryAck,
    MessageType::AscPullReq,
    MessageType::AscPullAck,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let collection = MessageCollection::default();
        assert_eq!(collection.len(), 0);
        assert!(collection.get(0).is_none());
    }

    #[test]
    fn add_message() {
        let mut collection = MessageCollection::default();
        collection.add(RecordedMessage::new_test_instance());
        assert_eq!(collection.len(), 1);
        assert!(collection.get(0).is_some());
    }

    #[test]
    fn clear() {
        let mut collection = MessageCollection::default();
        collection.add(RecordedMessage::new_test_instance());

        collection.clear();

        assert_eq!(collection.len(), 0);
        assert_eq!(collection.all_messages.len(), 0);
        assert_eq!(collection.filtered.len(), 0);
    }

    #[test]
    fn filter() {
        let mut collection = MessageCollection::default();

        let channel_id = ChannelId::from(2);
        let another_channel = ChannelId::from(3);

        let message1 = RecordedMessage {
            channel_id: another_channel,
            ..RecordedMessage::new_test_instance()
        };
        let message2 = RecordedMessage {
            channel_id,
            ..RecordedMessage::new_test_instance()
        };
        let message3 = RecordedMessage {
            channel_id: another_channel,
            ..RecordedMessage::new_test_instance()
        };
        let message4 = RecordedMessage {
            channel_id,
            ..RecordedMessage::new_test_instance()
        };
        collection.add(message1);
        collection.add(message2);
        collection.add(message3);
        collection.add(message4);

        collection.set_filter(MessageFilter::channel(channel_id));

        assert_eq!(collection.len(), 2);
        assert!(collection.get(0).is_some());
        assert!(collection.get(1).is_some());
        assert!(collection.get(2).is_none());
    }
}
