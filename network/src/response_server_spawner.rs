use crate::{DataReceiver, TcpChannelAdapter};
use std::sync::Arc;

/// Responsable for asynchronously launching a response server for a given channel
pub trait ResponseServerSpawner: Send + Sync {
    fn spawn(
        &self,
        channel_adapter: Arc<TcpChannelAdapter>,
        receiver: Box<dyn DataReceiver + Send>,
    );
}

pub struct NullResponseServerSpawner {}

impl NullResponseServerSpawner {
    pub fn new() -> Self {
        Self {}
    }
}

impl ResponseServerSpawner for NullResponseServerSpawner {
    fn spawn(
        &self,
        _channel_adapter: Arc<TcpChannelAdapter>,
        _receiver: Box<dyn DataReceiver + Send>,
    ) {
    }
}
