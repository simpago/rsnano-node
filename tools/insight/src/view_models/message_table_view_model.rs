use super::{MessageViewModel, PaletteColor};
use crate::message_collection::{MessageCollection, RecordedMessage};
use rsnano_core::{Account, BlockHash};
use rsnano_messages::{Message, MessageType};
use rsnano_network::ChannelDirection;
use std::sync::{Arc, RwLock};

pub(crate) struct RowViewModel {
    pub channel_id: String,
    pub direction: String,
    pub message: String,
    pub is_selected: bool,
    pub color: PaletteColor,
}

pub(crate) struct MessageTableViewModel {
    selected: Option<MessageViewModel>,
    selected_index: Option<usize>,
    messages: Arc<RwLock<MessageCollection>>,
    pub message_types: Vec<MessageTypeOptionViewModel>,
    pub hash_filter: String,
    pub hash_error: bool,
    pub account_filter: String,
    pub account_error: bool,
}

impl MessageTableViewModel {
    pub(crate) fn new(messages: Arc<RwLock<MessageCollection>>) -> Self {
        Self {
            messages,
            selected: None,
            selected_index: None,
            message_types: Vec::new(),
            account_filter: String::new(),
            account_error: false,
            hash_filter: String::new(),
            hash_error: false,
        }
    }

    pub(crate) fn heading(&self) -> String {
        format!("Messages ({})", self.messages.read().unwrap().len())
    }

    pub(crate) fn get_row(&self, index: usize) -> Option<RowViewModel> {
        let message = self.messages.read().unwrap().get(index)?;
        let message_text = message_summary_label(&message);

        Some(RowViewModel {
            channel_id: message.channel_id.to_string(),
            direction: if message.direction == ChannelDirection::Inbound {
                "in".into()
            } else {
                "out".into()
            },
            message: message_text,
            color: message_color(&message.message),
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

    pub(crate) fn update_type_filter(&self) {
        self.messages.write().unwrap().filter_message_types(
            self.message_types
                .iter()
                .filter(|i| i.selected)
                .map(|i| i.value),
        );
    }

    pub(crate) fn update_hash_filter(&mut self) {
        if self.hash_filter.trim().is_empty() {
            self.messages.write().unwrap().filter_hash(None);
            self.hash_error = false;
        } else if let Ok(hash) = BlockHash::decode_hex(self.hash_filter.trim()) {
            self.messages.write().unwrap().filter_hash(Some(hash));
            self.hash_error = false;
        } else {
            self.hash_error = true;
        }
    }

    pub(crate) fn update_account_filter(&mut self) {
        if self.account_filter.trim().is_empty() {
            self.messages.write().unwrap().filter_account(None);
            self.account_error = false;
        } else if let Ok(account) = Account::decode_account(self.account_filter.trim()) {
            self.messages.write().unwrap().filter_account(Some(account));
            self.account_error = false;
        } else if let Ok(account) = Account::decode_hex(self.account_filter.trim()) {
            self.messages.write().unwrap().filter_account(Some(account));
            self.account_error = false;
        } else {
            self.account_error = true;
        }
    }

    pub(crate) fn update_message_counts(&mut self) {
        let messages = self.messages.read().unwrap();
        let counts = messages.message_counts();
        let empty = Vec::with_capacity(counts.len());

        let old = std::mem::replace(&mut self.message_types, empty);
        for (msg_type, count) in counts {
            self.message_types.push(MessageTypeOptionViewModel {
                value: *msg_type,
                label: format!("{}({})", msg_type.as_str(), count),
                selected: false,
            })
        }

        for mut type_model in old {
            if type_model.selected {
                let mut found = false;
                for mt in self.message_types.iter_mut() {
                    if mt.value == type_model.value {
                        mt.selected = true;
                        found = true;
                        break;
                    }
                }

                if !found {
                    type_model.label = format!("{}({})", type_model.value.as_str(), 0);
                    self.message_types.push(type_model);
                }
            }
        }

        self.message_types.sort_by_key(|x| x.value as u8)
    }
}

pub(crate) struct MessageTypeOptionViewModel {
    pub value: MessageType,
    pub label: String,
    pub selected: bool,
}

// Define colors for different message types
fn message_color(message: &Message) -> PaletteColor {
    match message {
        // Important messages
        Message::Publish(_) => PaletteColor::Blue1,
        Message::ConfirmAck(_) => PaletteColor::Orange1,
        Message::ConfirmReq(_) => PaletteColor::Red1,

        Message::TelemetryAck(_) => PaletteColor::Purple1,
        Message::TelemetryReq => PaletteColor::Purple2,

        // Less important messages with refined grays
        Message::AscPullAck(_) => PaletteColor::Neutral2,
        Message::AscPullReq(_) => PaletteColor::Neutral3,

        // Other messages with neutral background
        _ => PaletteColor::Neutral4,
    }
}

fn message_summary_label(message: &RecordedMessage) -> String {
    match &message.message {
        Message::ConfirmAck(ack) => {
            let rebroadcast = if ack.is_rebroadcasted() { " (r)" } else { "" };
            let final_vote = if ack.vote().is_final() { " (f)" } else { "" };
            format!(
                "{:?} ({}){}{}",
                message.message.message_type(),
                ack.vote().hashes.len(),
                final_vote,
                rebroadcast
            )
        }
        Message::ConfirmReq(req) => {
            format!(
                "{:?} ({})",
                message.message.message_type(),
                req.roots_hashes().len()
            )
        }
        _ => format!("{:?}", message.message.message_type()),
    }
}
