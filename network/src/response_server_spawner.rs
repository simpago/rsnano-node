use crate::TcpChannelAdapter;
use std::sync::Arc;

/// Responsable for asynchronously launching a response server for a given channel
pub trait ResponseServerSpawner: Send + Sync {
    fn spawn(&self, channel_adapter: Arc<TcpChannelAdapter>);
}

pub struct NullResponseServerSpawner {}

impl NullResponseServerSpawner {
    pub fn new() -> Self {
        Self {}
    }
}

impl ResponseServerSpawner for NullResponseServerSpawner {
    fn spawn(&self, _channel_adapter: Arc<TcpChannelAdapter>) {}
}
