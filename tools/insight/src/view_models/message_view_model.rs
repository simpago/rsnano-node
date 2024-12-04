use crate::message_collection::RecordedMessage;
use rsnano_network::ChannelDirection;

#[derive(Clone)]
pub(crate) struct MessageViewModel {
    pub channel_id: String,
    pub direction: String,
    pub message_type: String,
    pub date: String,
    pub message: String,
}

impl From<RecordedMessage> for MessageViewModel {
    fn from(value: RecordedMessage) -> Self {
        Self {
            channel_id: value.channel_id.to_string(),
            direction: if value.direction == ChannelDirection::Inbound {
                "in".into()
            } else {
                "out".into()
            },
            date: value.date.to_string(),
            message_type: format!("{:?}", value.message.message_type()),
            message: format!("{:#?}", value.message),
        }
    }
}
