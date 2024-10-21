use crate::channels::Channels;
use rsnano_network::ChannelDirection;

pub(crate) struct ChannelsViewModel<'a>(&'a mut Channels);

impl<'a> ChannelsViewModel<'a> {
    pub(crate) fn new(channels: &'a mut Channels) -> Self {
        Self(channels)
    }

    pub(crate) fn get_row(&self, index: usize) -> Option<ChannelViewModel> {
        let channel = self.0.get(index)?;
        Some(ChannelViewModel {
            channel_id: channel.channel_id().to_string(),
            remote_addr: channel.peer_addr().to_string(),
            direction: match channel.direction() {
                ChannelDirection::Inbound => "in",
                ChannelDirection::Outbound => "out",
            },
            is_selected: self.0.selected_index() == Some(index),
        })
    }

    pub(crate) fn channel_count(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn select(&mut self, index: usize) {
        self.0.select_index(index);
    }

    pub(crate) fn heading(&self) -> String {
        format!("Channels ({})", self.0.len())
    }
}

pub(crate) struct ChannelViewModel {
    pub channel_id: String,
    pub remote_addr: String,
    pub direction: &'static str,
    pub is_selected: bool,
}
