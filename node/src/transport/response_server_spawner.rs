use super::{nano_data_receiver_factory::DataReceiverFactory, response_server::ResponseServer};
use rsnano_network::{ResponseServerSpawner, TcpChannelAdapter};
use std::sync::Arc;

pub struct NanoResponseServerSpawner {
    pub(crate) tokio: tokio::runtime::Handle,
    pub(crate) data_receiver_factory: Box<dyn DataReceiverFactory + Send + Sync>,
}

impl ResponseServerSpawner for NanoResponseServerSpawner {
    fn spawn(&self, channel_adapter: Arc<TcpChannelAdapter>) {
        let channel = channel_adapter.channel.clone();
        let server = Arc::new(ResponseServer::new(channel_adapter));
        let mut receiver = self.data_receiver_factory.create_receiver_for(channel);
        receiver.initialize();
        self.tokio.spawn(async move { server.run(receiver).await });
    }
}
