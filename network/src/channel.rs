use crate::{
    bandwidth_limiter::BandwidthLimiter,
    utils::into_ipv6_socket_address,
    write_queue::{WriteQueue, WriteQueueReceiver},
    AsyncBufferReader, ChannelDirection, ChannelId, ChannelInfo, DropPolicy, NetworkObserver,
    NullNetworkObserver, TrafficType, WriteQueueAdapter,
};
use async_trait::async_trait;
use rsnano_core::utils::{TEST_ENDPOINT_1, TEST_ENDPOINT_2};
use rsnano_nullable_clock::{SteadyClock, Timestamp};
use rsnano_nullable_tcp::TcpStream;
use std::{
    fmt::Display,
    net::{Ipv6Addr, SocketAddrV6},
    sync::{Arc, Weak},
    time::Duration,
};
use tokio::{select, time::sleep};
use tokio_util::sync::CancellationToken;
use tracing::debug;

pub struct Channel {
    channel_id: ChannelId,
    pub info: Arc<ChannelInfo>,
    limiter: Arc<BandwidthLimiter>,
    write_queue: Arc<WriteQueue>,
    stream: Weak<TcpStream>,
    clock: Arc<SteadyClock>,
    observer: Arc<dyn NetworkObserver>,
    cancel_token: CancellationToken,
}

impl Channel {
    const MAX_QUEUE_SIZE: usize = 128;

    fn new(
        channel_info: Arc<ChannelInfo>,
        stream: Weak<TcpStream>,
        limiter: Arc<BandwidthLimiter>,
        clock: Arc<SteadyClock>,
        observer: Arc<dyn NetworkObserver>,
        cancel_token: CancellationToken,
    ) -> (Self, WriteQueueReceiver) {
        let (write_queue, receiver) = WriteQueue::new(Self::MAX_QUEUE_SIZE);

        let channel = Self {
            channel_id: channel_info.channel_id(),
            info: channel_info,
            limiter,
            write_queue: Arc::new(write_queue),
            stream,
            clock,
            observer,
            cancel_token,
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
            Arc::downgrade(&Arc::new(TcpStream::new_null())),
            Arc::new(BandwidthLimiter::default()),
            Arc::new(SteadyClock::new_null()),
            Arc::new(NullNetworkObserver::new()),
            CancellationToken::new(),
        );
        channel
    }

    pub fn create(
        channel_info: Arc<ChannelInfo>,
        stream: TcpStream,
        limiter: Arc<BandwidthLimiter>,
        clock: Arc<SteadyClock>,
        observer: Arc<dyn NetworkObserver>,
        handle: &tokio::runtime::Handle,
    ) -> Arc<Self> {
        let stream = Arc::new(stream);
        let stream_l = stream.clone();
        let info = channel_info.clone();
        let cancel_token = CancellationToken::new();
        let (channel, mut receiver) = Self::new(
            channel_info,
            Arc::downgrade(&stream),
            limiter,
            clock.clone(),
            observer.clone(),
            cancel_token.clone(),
        );

        let write_queue = Arc::downgrade(&channel.write_queue);
        info.set_write_queue(Box::new(WriteQueueAdapterImpl {
            queue: write_queue,
            cancel_token: cancel_token.clone(),
        }));

        // process write queue:
        handle.spawn(async move {
            loop {
                let res = select! {
                    _ = cancel_token.cancelled() =>{
                        return;
                    },
                  res = receiver.pop() => res
                };

                if let Some((entry, _)) = res {
                    let mut written = 0;
                    let buffer = &entry.buffer;
                    loop {
                        select! {
                            _ = cancel_token.cancelled() =>{
                                return;
                            }
                            res = stream_l.writable() =>{
                            match res {
                            Ok(()) => match stream_l.try_write(&buffer[written..]) {
                                Ok(n) => {
                                    written += n;
                                    if written >= buffer.len() {
                                        observer.send_succeeded(written);
                                        info.set_last_activity(clock.now());
                                        break;
                                    }
                                }
                                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                    continue;
                                }
                                Err(_) => {
                                    info.close();
                                    return;
                                }
                            },
                            Err(_) => {
                                info.close();
                                return;
                            }
                        }
                            }
                        }
                    }
                } else {
                    break;
                }
            }
            info.close();
        });

        let channel = Arc::new(channel);
        let channel_l = channel.clone();
        handle.spawn(async move { channel_l.ongoing_checkup().await });
        channel
    }

    pub fn channel_id(&self) -> ChannelId {
        self.channel_id
    }

    pub fn local_addr(&self) -> SocketAddrV6 {
        let no_addr = SocketAddrV6::new(Ipv6Addr::LOCALHOST, 0, 0, 0);
        let Some(stream) = self.stream.upgrade() else {
            return no_addr;
        };

        stream
            .local_addr()
            .map(|addr| into_ipv6_socket_address(addr))
            .unwrap_or(no_addr)
    }

    pub async fn send_buffer(
        &self,
        buffer: &[u8],
        traffic_type: TrafficType,
    ) -> anyhow::Result<()> {
        if self.info.is_closed() {
            bail!("socket closed");
        }

        while self.info.is_queue_full(traffic_type) {
            // TODO: better implementation
            sleep(Duration::from_millis(20)).await;
        }

        while !self.limiter.should_pass(buffer.len(), traffic_type) {
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
            self.info.set_last_activity(self.clock.now());
        } else {
            self.observer.send_failed();
            debug!(channel_id = %self.channel_id(), remote_addr = ?self.info.peer_addr(), "Closing channel after write error");
            self.info.close();
        }

        result?;
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

        let should_pass = self.limiter.should_pass(buffer.len(), traffic_type);
        if !should_pass && drop_policy == DropPolicy::CanDrop {
            return false;
        } else {
            // TODO notify bandwidth limiter that we are sending it anyway
        }

        let (inserted, write_error) = self
            .write_queue
            .try_insert(Arc::new(buffer.to_vec()), traffic_type); // TODO don't copy into vec. Split into fixed size packets

        if write_error {
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

        let Some(stream) = self.stream.upgrade() else {
            return Err(anyhow!("TCP stream dropped"));
        };

        let mut read = 0;
        loop {
            let res = select! {
                _  = self.cancel_token.cancelled() =>{
                    return Err(anyhow!("cancelled"));
                },
                res = stream.readable() => res
            };
            match res {
                Ok(_) => {
                    match stream.try_read(&mut buffer[read..count]) {
                        Ok(0) => {
                            self.observer.read_failed();
                            self.info.close();
                            return Err(anyhow!("remote side closed the channel"));
                        }
                        Ok(n) => {
                            read += n;
                            if read >= count {
                                self.observer.read_succeeded(count);
                                self.info.set_last_activity(self.clock.now());
                                return Ok(());
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            continue;
                        }
                        Err(e) => {
                            self.observer.read_failed();
                            self.info.close();
                            return Err(e.into());
                        }
                    };
                }
                Err(e) => {
                    self.observer.read_failed();
                    self.info.close();
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

struct WriteQueueAdapterImpl {
    queue: Weak<WriteQueue>,
    cancel_token: CancellationToken,
}

impl WriteQueueAdapter for WriteQueueAdapterImpl {
    fn is_queue_full(&self, traffic_type: TrafficType) -> bool {
        match self.queue.upgrade() {
            Some(queue) => queue.capacity(traffic_type) <= Channel::MAX_QUEUE_SIZE,
            None => true,
        }
    }

    fn close(&self) {
        self.cancel_token.cancel();
    }
}
