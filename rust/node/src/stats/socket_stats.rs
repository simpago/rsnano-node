use super::{DetailType, Direction, StatType, Stats};
use crate::transport::{ChannelDirection, SocketObserver};
use std::sync::Arc;

pub struct SocketStats {
    stats: Arc<Stats>,
}

impl SocketStats {
    pub fn new(stats: Arc<Stats>) -> Self {
        Self { stats }
    }
}

impl SocketObserver for SocketStats {
    fn inactive_connection_dropped(&self, direction: ChannelDirection) {
    }
}
