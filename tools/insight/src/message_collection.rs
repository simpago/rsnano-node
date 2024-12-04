use chrono::{DateTime, TimeZone, Utc};
use rsnano_core::{Account, BlockHash};
use rsnano_messages::{AscPullAckType, AscPullReqType, HashType, Message, MessageType};
use rsnano_network::{ChannelDirection, ChannelId};
use std::collections::HashMap;

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

#[derive(Default, PartialEq, Eq, Debug, Clone)]
pub(crate) struct MessageFilter {
    channel_id: Option<ChannelId>,
    hash: Option<BlockHash>,
    account: Option<Account>,
    types: Vec<MessageType>,
}

impl MessageFilter {
    pub fn all() -> Self {
        Default::default()
    }

    pub fn channel(channel_id: ChannelId) -> Self {
        Self {
            channel_id: Some(channel_id),
            ..Default::default()
        }
    }

    pub fn include(&self, message: &RecordedMessage) -> bool {
        self.include_channel(message)
            && self.include_message_type(message)
            && self.include_message_content(message)
    }

    pub fn with_types(&self, types: Vec<MessageType>) -> Self {
        Self {
            types,
            ..self.clone()
        }
    }

    pub fn with_channel(&self, channel_id: Option<ChannelId>) -> Self {
        Self {
            channel_id,
            ..self.clone()
        }
    }

    pub fn with_hash(&self, hash: Option<BlockHash>) -> Self {
        Self {
            hash,
            ..self.clone()
        }
    }

    pub fn with_account(&self, account: Option<Account>) -> Self {
        Self {
            account,
            ..self.clone()
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

    fn include_message_content(&self, message: &RecordedMessage) -> bool {
        self.include_hash(message) && self.include_account(message)
    }

    fn include_hash(&self, message: &RecordedMessage) -> bool {
        let Some(hash) = self.hash else {
            return true;
        };
        match &message.message {
            Message::Publish(i) => i.block.hash() == hash,
            Message::AscPullAck(ack) => match &ack.pull_type {
                AscPullAckType::Blocks(i) => i.blocks().iter().any(|b| b.hash() == hash),
                AscPullAckType::AccountInfo(i) => {
                    i.account_open == hash
                        || i.account_head == hash
                        || i.account_conf_frontier == hash
                }
                AscPullAckType::Frontiers(i) => i.iter().any(|f| f.hash == hash),
            },
            Message::AscPullReq(req) => match &req.req_type {
                AscPullReqType::Blocks(i) => {
                    i.start_type == HashType::Block && hash == i.start.into()
                }
                AscPullReqType::AccountInfo(i) => {
                    i.target_type == HashType::Block && hash == i.target.into()
                }
                AscPullReqType::Frontiers(_) => false,
            },
            Message::ConfirmAck(i) => i.vote().hashes.iter().any(|h| *h == hash),
            Message::ConfirmReq(i) => i.roots_hashes().iter().any(|(h, _)| *h == hash),
            _ => false,
        }
    }

    fn include_account(&self, message: &RecordedMessage) -> bool {
        let Some(account) = self.account else {
            return true;
        };

        match &message.message {
            Message::Publish(i) => i.block.account_field() == Some(account),
            Message::AscPullAck(ack) => match &ack.pull_type {
                AscPullAckType::Blocks(i) => i
                    .blocks()
                    .iter()
                    .any(|b| b.account_field() == Some(account)),
                AscPullAckType::AccountInfo(i) => i.account == account,
                AscPullAckType::Frontiers(i) => i.iter().any(|f| f.account == account),
            },
            Message::AscPullReq(req) => match &req.req_type {
                AscPullReqType::Blocks(i) => {
                    i.start_type == HashType::Account && account == i.start.into()
                }
                AscPullReqType::AccountInfo(i) => {
                    i.target_type == HashType::Account && account == i.target.into()
                }
                AscPullReqType::Frontiers(i) => i.start == account,
            },
            Message::BulkPullAccount(i) => i.account == account,
            Message::ConfirmAck(ack) => ack.vote().voting_account == account.into(),
            Message::FrontierReq(i) => i.start == account,
            _ => false,
        }
    }
}

#[derive(Default)]
pub(crate) struct MessageCollection {
    all_messages: Vec<RecordedMessage>,
    filtered: Vec<RecordedMessage>,
    filter: MessageFilter,
    message_counts: HashMap<MessageType, usize>,
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
        if self.filter.include_channel(&message) {
            *self
                .message_counts
                .entry(message.message.message_type())
                .or_default() += 1;
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

    pub fn message_counts(&self) -> &HashMap<MessageType, usize> {
        &self.message_counts
    }

    pub fn filter_channel(&mut self, channel_id: Option<ChannelId>) {
        self.set_filter(self.filter.with_channel(channel_id))
    }

    pub fn filter_message_types(&mut self, types: impl IntoIterator<Item = MessageType>) {
        let types = types.into_iter().collect();
        self.set_filter(self.filter.with_types(types));
    }

    pub fn filter_hash(&mut self, hash: Option<BlockHash>) {
        self.set_filter(self.filter.with_hash(hash));
    }

    pub fn filter_account(&mut self, hash: Option<Account>) {
        self.set_filter(self.filter.with_account(hash));
    }

    fn set_filter(&mut self, filter: MessageFilter) {
        self.filter = filter;
        self.filtered = self
            .all_messages
            .iter()
            .filter(|m| self.filter.include(m))
            .cloned()
            .collect();
        self.message_counts.clear();

        for m in self
            .all_messages
            .iter()
            .filter(|m| self.filter.include_channel(m))
        {
            if self.filter.include_message_content(m) {
                *self
                    .message_counts
                    .entry(m.message.message_type())
                    .or_default() += 1;
            }
        }
    }
}

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
