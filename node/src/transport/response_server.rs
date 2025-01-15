use super::nano_data_receiver_factory::DataReceiver;
use rsnano_network::{Channel, TcpChannelAdapter};
use std::sync::Arc;
use tracing::debug;

pub(crate) struct ResponseServer {
    channel_adapter: Arc<TcpChannelAdapter>,
    channel: Arc<Channel>,
}

impl ResponseServer {
    pub fn new(channel_adapter: Arc<TcpChannelAdapter>) -> Self {
        let channel = channel_adapter.channel.clone();
        Self {
            channel,
            channel_adapter,
        }
    }

    pub async fn run(&self, mut receiver: Box<dyn DataReceiver + Send>) {
        let mut buffer = [0u8; 1024];

        loop {
            // TODO: abort readable waiting if channel closed

            if let Err(e) = self.channel_adapter.readable().await {
                debug!(
                    "Error reading buffer: {:?} ({})",
                    e,
                    self.channel.peer_addr()
                );
                self.channel.close();
                return;
            }

            let read_count = match self.channel_adapter.try_read(&mut buffer) {
                Ok(n) => n,
                Err(e) => {
                    debug!(
                        "Error reading buffer: {:?} ({})",
                        e,
                        self.channel.peer_addr()
                    );
                    self.channel.close();
                    return;
                }
            };

            let new_data = &buffer[..read_count];

            if !receiver.receive(new_data) {
                break;
            }
        }
    }
}
