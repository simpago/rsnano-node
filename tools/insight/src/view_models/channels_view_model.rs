use crate::channels::{Channels, RepState};
use num_format::{Locale, ToFormattedString};
use rsnano_network::ChannelDirection;

pub(crate) struct ChannelsViewModel<'a>(&'a mut Channels);

impl<'a> ChannelsViewModel<'a> {
    pub(crate) fn new(channels: &'a mut Channels) -> Self {
        Self(channels)
    }

    pub(crate) fn get_row(&self, index: usize) -> Option<ChannelViewModel> {
        let channel = self.0.get(index)?;
        let mut result = ChannelViewModel {
            channel_id: channel.channel_id.to_string(),
            remote_addr: channel.remote_addr.to_string(),
            direction: match channel.direction {
                ChannelDirection::Inbound => "in",
                ChannelDirection::Outbound => "out",
            },
            is_selected: self.0.selected_index() == Some(index),
            block_count: String::new(),
            cemented_count: String::new(),
            unchecked_count: String::new(),
            maker: "",
            version: String::new(),
            bandwidth_cap: String::new(),
            rep_weight: channel.rep_weight.format_balance(0),
            rep_state: channel.rep_state,
        };

        if let Some(telemetry) = &channel.telemetry {
            result.block_count = telemetry.block_count.to_formatted_string(&Locale::en);
            result.cemented_count = telemetry.cemented_count.to_formatted_string(&Locale::en);
            result.unchecked_count = telemetry.unchecked_count.to_formatted_string(&Locale::en);
            result.maker = match telemetry.maker {
                0 | 1 => "NF",
                3 => "RsNano",
                _ => "unknown",
            };
            result.version = format!(
                "v{}.{}.{}",
                telemetry.major_version, telemetry.minor_version, telemetry.patch_version
            );
            result.bandwidth_cap = format!("{}mb/s", telemetry.bandwidth_cap / (1024 * 1024))
        }

        Some(result)
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
    pub block_count: String,
    pub cemented_count: String,
    pub unchecked_count: String,
    pub maker: &'static str,
    pub version: String,
    pub bandwidth_cap: String,
    pub rep_weight: String,
    pub rep_state: RepState,
}
