use crate::{transport::ChannelId, utils::Timestamp};
use rsnano_core::Account;

/// A representative to which we have a direct connection
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeeredRep {
    pub account: Account,
    pub channel_id: ChannelId,
    pub last_request: Timestamp,
}

impl PeeredRep {
    pub fn new(account: Account, channel_id: ChannelId, last_request: Timestamp) -> Self {
        Self {
            account,
            channel_id,
            last_request,
        }
    }
}
