use rsnano_core::PublicKey;
use rsnano_network::ChannelId;
use rsnano_nullable_clock::Timestamp;

/// A representative to which we have a direct connection
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeeredRep {
    pub account: PublicKey,
    pub channel_id: ChannelId,
    pub last_request: Timestamp,
}

impl PeeredRep {
    pub fn new(account: PublicKey, channel_id: ChannelId, last_request: Timestamp) -> Self {
        Self {
            account,
            channel_id,
            last_request,
        }
    }
}
