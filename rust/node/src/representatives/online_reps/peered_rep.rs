use std::time::Duration;

use crate::transport::ChannelId;
use rsnano_core::Account;

/// A representative to which we have a direct connection
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeeredRep {
    pub account: Account,
    pub channel_id: ChannelId,
    pub last_request: Duration,
}

impl PeeredRep {
    pub fn new(account: Account, channel_id: ChannelId, last_request: Duration) -> Self {
        Self {
            account,
            channel_id,
            last_request,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn new_test_instance() -> Self {
        Self::new(Account::from(42), 123.into(), Duration::from_secs(1))
    }
}
