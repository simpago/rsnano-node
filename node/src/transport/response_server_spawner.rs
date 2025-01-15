use rsnano_network::{DataReceiver, ResponseServerSpawner, TcpChannelAdapter};
use std::sync::Arc;
use tracing::debug;

pub struct NanoResponseServerSpawner {
    pub(crate) tokio: tokio::runtime::Handle,
}

impl ResponseServerSpawner for NanoResponseServerSpawner {
    fn spawn(
        &self,
        channel_adapter: Arc<TcpChannelAdapter>,
        mut receiver: Box<dyn DataReceiver + Send>,
    ) {
        let channel = channel_adapter.channel.clone();

        self.tokio.spawn(async move {
            let mut buffer = [0u8; 1024];

            loop {
                // TODO: abort readable waiting if channel closed

                if let Err(e) = channel_adapter.readable().await {
                    debug!("Error reading buffer: {:?} ({})", e, channel.peer_addr());
                    channel.close();
                    return;
                }

                let read_count = match channel_adapter.try_read(&mut buffer) {
                    Ok(n) => n,
                    Err(e) => {
                        debug!("Error reading buffer: {:?} ({})", e, channel.peer_addr());
                        channel.close();
                        return;
                    }
                };

                let new_data = &buffer[..read_count];

                if !receiver.receive(new_data) {
                    break;
                }
            }
        });
    }
}
