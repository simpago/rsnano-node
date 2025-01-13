use crate::ChannelAdapter;
use std::sync::Arc;

/// Responsable for asynchronously launching a response server for a given channel
pub trait ResponseServerSpawner: Send + Sync {
    fn spawn(&self, channel: Arc<ChannelAdapter>);
}

pub struct NullResponseServerSpawner {}

impl NullResponseServerSpawner {
    pub fn new() -> Self {
        Self {}
    }
}

impl ResponseServerSpawner for NullResponseServerSpawner {
    fn spawn(&self, _channel: Arc<ChannelAdapter>) {}
}
