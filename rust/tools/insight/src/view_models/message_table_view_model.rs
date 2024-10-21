use super::MessageViewModel;
use crate::message_collection::{MessageCollection, AVALABLE_FILTER_TYPES};
use rsnano_messages::MessageType;
use rsnano_network::ChannelDirection;
use std::sync::{Arc, RwLock};

pub(crate) struct RowViewModel {
    pub channel_id: String,
    pub direction: String,
    pub message: String,
    pub is_selected: bool,
}

pub(crate) struct MessageTableViewModel {
    selected: Option<MessageViewModel>,
    selected_index: Option<usize>,
    messages: Arc<RwLock<MessageCollection>>,
    pub message_types: Vec<MessageTypeOptionViewModel>,
}

impl MessageTableViewModel {
    pub(crate) fn new(messages: Arc<RwLock<MessageCollection>>) -> Self {
        Self {
            messages,
            selected: None,
            selected_index: None,
            message_types: AVALABLE_FILTER_TYPES
                .iter()
                .map(|t| MessageTypeOptionViewModel {
                    value: *t,
                    name: t.as_str(),
                    selected: false,
                })
                .collect(),
        }
    }

    pub(crate) fn heading(&self) -> String {
        format!("Messages ({})", self.messages.read().unwrap().len())
    }

    pub(crate) fn get_row(&self, index: usize) -> Option<RowViewModel> {
        let message = self.messages.read().unwrap().get(index)?;
        Some(RowViewModel {
            channel_id: message.channel_id.to_string(),
            direction: if message.direction == ChannelDirection::Inbound {
                "in".into()
            } else {
                "out".into()
            },
            message: format!("{:?}", message.message.message_type()),
            is_selected: self.selected_index == Some(index),
        })
    }

    pub(crate) fn message_count(&self) -> usize {
        self.messages.read().unwrap().len()
    }

    pub(crate) fn selected_message(&self) -> Option<MessageViewModel> {
        self.selected.clone()
    }

    pub(crate) fn select_message(&mut self, index: usize) {
        let message = self.messages.read().unwrap().get(index).unwrap();
        self.selected = Some(message.into());
        self.selected_index = Some(index);
    }

    pub(crate) fn update_filter(&self) {
        self.messages.write().unwrap().filter_message_types(
            self.message_types
                .iter()
                .filter(|i| i.selected)
                .map(|i| i.value),
        );
    }
}

pub(crate) struct MessageTypeOptionViewModel {
    pub value: MessageType,
    pub name: &'static str,
    pub selected: bool,
}
