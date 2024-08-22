use crate::{
    bandwidth_limiter::OutboundBandwidthLimiter,
    utils::into_ipv6_socket_address,
    write_queue::{WriteQueue, WriteQueueReceiver},
    AsyncBufferReader, ChannelDirection, ChannelId, ChannelInfo, DropPolicy, NetworkInfo,
    NetworkObserver, NullNetworkObserver, TrafficType,
};
use async_trait::async_trait;
use rsnano_core::{
    utils::{TEST_ENDPOINT_1, TEST_ENDPOINT_2},
    PublicKey,
};
use rsnano_nullable_clock::{SteadyClock, Timestamp};
use rsnano_nullable_tcp::TcpStream;
use std::{
    fmt::Display,
    net::{Ipv6Addr, SocketAddrV6},
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::time::sleep;
use tracing::debug;

pub struct Channel {
    channel_id: ChannelId,
    network_info: Arc<RwLock<NetworkInfo>>,
    pub info: Arc<ChannelInfo>,
    limiter: Arc<OutboundBandwidthLimiter>,
    write_queue: Arc<WriteQueue>,
    stream: Arc<TcpStream>,
    clock: Arc<SteadyClock>,
    observer: Arc<dyn NetworkObserver>,
}

impl Channel {
    const MAX_QUEUE_SIZE: usize = 128;

    fn new(
        channel_info: Arc<ChannelInfo>,
        network_info: Arc<RwLock<NetworkInfo>>,
        stream: Arc<TcpStream>,
        limiter: Arc<OutboundBandwidthLimiter>,
        clock: Arc<SteadyClock>,
        observer: Arc<dyn NetworkObserver>,
    ) -> (Self, WriteQueueReceiver) {
        let (write_queue, receiver) = WriteQueue::new(Self::MAX_QUEUE_SIZE);

        let channel = Self {
            channel_id: channel_info.channel_id(),
            info: channel_info,
            network_info,
            limiter,
            write_queue: Arc::new(write_queue),
            stream,
            clock,
            observer,
        };

        (channel, receiver)
    }

    pub fn new_null() -> Self {
        Self::new_null_with_id(42)
    }

    pub fn new_null_with_id(id: impl Into<ChannelId>) -> Self {
        let channel_id = id.into();
        let (channel, _receiver) = Self::new(
            Arc::new(ChannelInfo::new(
                channel_id,
                TEST_ENDPOINT_1,
                TEST_ENDPOINT_2,
                ChannelDirection::Outbound,
                u8::MAX,
                Timestamp::new_test_instance(),
            )),
            Arc::new(RwLock::new(NetworkInfo::new_test_instance())),
            Arc::new(TcpStream::new_null()),
            Arc::new(OutboundBandwidthLimiter::default()),
            Arc::new(SteadyClock::new_null()),
            Arc::new(NullNetworkObserver::new()),
        );
        channel
    }

    pub async fn create(
        channel_info: Arc<ChannelInfo>,
        stream: TcpStream,
        limiter: Arc<OutboundBandwidthLimiter>,
        network_info: Arc<RwLock<NetworkInfo>>,
        clock: Arc<SteadyClock>,
        observer: Arc<dyn NetworkObserver>,
    ) -> Arc<Self> {
        let stream = Arc::new(stream);
        let stream_l = stream.clone();
        let info = channel_info.clone();
        let (channel, mut receiver) =
            Self::new(channel_info, network_info, stream, limiter, clock, observer);

        let write_queue = Arc::downgrade(&channel.write_queue);
        info.set_queue_full_query(Box::new(move |traffic_type| {
            let Some(queue) = write_queue.upgrade() else {
                return true;
            };
            queue.capacity(traffic_type) <= Self::MAX_QUEUE_SIZE
        }));

        // process write queue:
        tokio::spawn(async move {
            while let Some((entry, _)) = receiver.pop().await {
                let mut written = 0;
                let buffer = &entry.buffer;
                loop {
                    match stream_l.writable().await {
                        Ok(()) => match stream_l.try_write(&buffer[written..]) {
                            Ok(n) => {
                                written += n;
                                if written >= buffer.len() {
                                    break;
                                }
                            }
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                continue;
                            }
                            Err(_) => {
                                break;
                            }
                        },
                        Err(_) => {
                            break;
                        }
                    }
                }
            }
            info.close();
        });

        let channel = Arc::new(channel);
        let channel_l = channel.clone();
        tokio::spawn(async move { channel_l.ongoing_checkup().await });
        channel
    }

    fn update_last_activity(&self) {
        self.info.set_last_activity(self.clock.now());
    }

    pub fn channel_id(&self) -> ChannelId {
        self.channel_id
    }

    pub fn set_node_id(&self, id: PublicKey) {
        self.network_info
            .read()
            .unwrap()
            .set_node_id(self.channel_id, id);
    }

    pub fn local_addr(&self) -> SocketAddrV6 {
        self.stream
            .local_addr()
            .map(|addr| into_ipv6_socket_address(addr))
            .unwrap_or(SocketAddrV6::new(Ipv6Addr::LOCALHOST, 0, 0, 0))
    }

    pub async fn send_buffer(
        &self,
        buffer: &[u8],
        traffic_type: TrafficType,
    ) -> anyhow::Result<()> {
        while self.info.is_queue_full(traffic_type) {
            // TODO: better implementation
            sleep(Duration::from_millis(20)).await;
        }

        while !self.limiter.should_pass(buffer.len(), traffic_type.into()) {
            // TODO: better implementation
            sleep(Duration::from_millis(20)).await;
        }

        if self.info.is_closed() {
            bail!("socket closed");
        }

        let buf_size = buffer.len();

        let result = self
            .write_queue
            .insert(Arc::new(buffer.to_vec()), traffic_type) // TODO don't copy into vec. Split into fixed size packets
            .await;

        if result.is_ok() {
            self.observer.send_succeeded(buf_size);
            self.update_last_activity();
            self.info.set_last_packet_sent(self.clock.now());
        } else {
            self.observer.send_failed();
            debug!(channel_id = %self.channel_id(), remote_addr = ?self.info.peer_addr(), "Closing channel after write error");
            self.info.close();
        }

        result?;

        self.info.set_last_packet_sent(self.clock.now());
        Ok(())
    }

    pub fn try_send_buffer(
        &self,
        buffer: &[u8],
        drop_policy: DropPolicy,
        traffic_type: TrafficType,
    ) -> bool {
        if self.info.is_closed() {
            return false;
        }

        if drop_policy == DropPolicy::CanDrop && self.info.is_queue_full(traffic_type) {
            return false;
        }

        let should_pass = self.limiter.should_pass(buffer.len(), traffic_type.into());
        if !should_pass && drop_policy == DropPolicy::CanDrop {
            return false;
        } else {
            // TODO notify bandwidth limiter that we are sending it anyway
        }

        let buf_size = buffer.len();

        let (inserted, write_error) = self
            .write_queue
            .try_insert(Arc::new(buffer.to_vec()), traffic_type); // TODO don't copy into vec. Split into fixed size packets

        if inserted {
            //TODO raise event when actually written to stream
            self.observer.send_succeeded(buf_size);
            self.update_last_activity();
            self.info.set_last_packet_sent(self.clock.now());
        } else if write_error {
            self.observer.send_failed();
            self.info.close();
            debug!(peer_addr = ?self.info.peer_addr(), channel_id = %self.channel_id(), mode = ?self.info.mode(), "Closing socket after write error");
        }
        inserted
    }

    async fn ongoing_checkup(&self) {
        loop {
            sleep(Duration::from_secs(2)).await;
            // If the socket is already dead, close just in case, and stop doing checkups
            if !self.info.is_alive() {
                return;
            }

            let now = self.clock.now();

            // if there is no activity for timeout seconds then disconnect
            let has_timed_out = (now - self.info.last_activity()) > self.info.timeout();
            if has_timed_out {
                self.observer.channel_timed_out(&self.info);
                self.info.set_timed_out(true);
                self.info.close();
            }
        }
    }
}

impl Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.info.peer_addr().fmt(f)
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        self.info.close();
    }
}

#[async_trait]
impl AsyncBufferReader for Channel {
    async fn read(&self, buffer: &mut [u8], count: usize) -> anyhow::Result<()> {
        if count > buffer.len() {
            return Err(anyhow!("buffer is too small for read count"));
        }

        if self.info.is_closed() {
            return Err(anyhow!("Tried to read from a closed TcpStream"));
        }

        let mut read = 0;
        loop {
            match self.stream.readable().await {
                Ok(_) => {
                    match self.stream.try_read(&mut buffer[read..count]) {
                        Ok(0) => {
                            self.observer.read_failed();
                            return Err(anyhow!("remote side closed the channel"));
                        }
                        Ok(n) => {
                            read += n;
                            if read >= count {
                                self.observer.read_succeeded(count);
                                self.update_last_activity();
                                self.info.set_last_packet_received(self.clock.now());
                                return Ok(());
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            continue;
                        }
                        Err(e) => {
                            self.observer.read_failed();
                            return Err(e.into());
                        }
                    };
                }
                Err(e) => {
                    self.observer.read_failed();
                    return Err(e.into());
                }
            }
        }
    }
}

pub struct ChannelReader(Arc<Channel>);

impl ChannelReader {
    pub fn new(channel: Arc<Channel>) -> Self {
        Self(channel)
    }
}

#[async_trait]
impl AsyncBufferReader for ChannelReader {
    async fn read(&self, buffer: &mut [u8], count: usize) -> anyhow::Result<()> {
        self.0.read(buffer, count).await
    }
}
