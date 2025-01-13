use crate::{
    bandwidth_limiter::BandwidthLimiter, utils::into_ipv6_socket_address, AsyncBufferReader,
    ChannelDirection, ChannelId, ChannelInfo, DropPolicy, NullNetworkObserver, TrafficType,
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

pub struct Channel {
    channel_id: ChannelId,
    pub info: Arc<ChannelInfo>,
    stream: Weak<TcpStream>,
    clock: Arc<SteadyClock>,
}

impl Channel {
    fn new(
        channel_info: Arc<ChannelInfo>,
        stream: Weak<TcpStream>,
        clock: Arc<SteadyClock>,
    ) -> Self {
        Self {
            channel_id: channel_info.channel_id(),
            info: channel_info,
            stream,
            clock,
        }
    }

    pub fn new_null() -> Self {
        Self::new_null_with_id(42)
    }

    pub fn new_null_with_id(id: impl Into<ChannelId>) -> Self {
        let channel_id = id.into();
        let channel = Self::new(
            Arc::new(ChannelInfo::new(
                channel_id,
                TEST_ENDPOINT_1,
                TEST_ENDPOINT_2,
                ChannelDirection::Outbound,
                u8::MAX,
                Timestamp::new_test_instance(),
                Arc::new(BandwidthLimiter::default()),
                Arc::new(NullNetworkObserver::new()),
            )),
            Arc::downgrade(&Arc::new(TcpStream::new_null())),
            Arc::new(SteadyClock::new_null()),
        );
        channel
    }

    pub fn create(
        channel_info: Arc<ChannelInfo>,
        stream: TcpStream,
        clock: Arc<SteadyClock>,
        handle: &tokio::runtime::Handle,
    ) -> Arc<Self> {
        let stream = Arc::new(stream);
        let info = channel_info.clone();
        let channel = Self::new(channel_info.clone(), Arc::downgrade(&stream), clock.clone());

        // process write queue:
        handle.spawn(async move {
            loop {
                let res = select! {
                    _ = info.cancelled() =>{
                        return;
                    },
                  res = channel_info.pop() => res
                };

                if let Some(entry) = res {
                    let mut written = 0;
                    let buffer = &entry.buffer;
                    loop {
                        select! {
                            _ = info.cancelled() =>{
                                return;
                            }
                            res = stream.writable() =>{
                            match res {
                            Ok(()) => match stream.try_write(&buffer[written..]) {
                                Ok(n) => {
                                    written += n;
                                    if written >= buffer.len() {
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

    // TODO delete and use channel_info directly
    pub async fn send_buffer(
        &self,
        buffer: &[u8],
        traffic_type: TrafficType,
    ) -> anyhow::Result<()> {
        self.info.send_buffer(buffer, traffic_type).await?;
        Ok(())
    }

    // TODO delete and use channel_info directly
    pub fn try_send_buffer(
        &self,
        buffer: &[u8],
        drop_policy: DropPolicy,
        traffic_type: TrafficType,
    ) -> bool {
        self.info.try_send_buffer(buffer, drop_policy, traffic_type)
    }

    async fn ongoing_checkup(&self) {
        loop {
            sleep(Duration::from_secs(2)).await;
            let now = self.clock.now();
            let timed_out = self.info.check_timeout(now);
            if timed_out {
                break;
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
                _  = self.info.cancelled() =>{
                    return Err(anyhow!("cancelled"));
                },
                res = stream.readable() => res
            };
            match res {
                Ok(_) => {
                    match stream.try_read(&mut buffer[read..count]) {
                        Ok(0) => {
                            self.info.read_failed();
                            return Err(anyhow!("remote side closed the channel"));
                        }
                        Ok(n) => {
                            read += n;
                            if read >= count {
                                self.info.read_succeeded(count, self.clock.now());
                                return Ok(());
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            continue;
                        }
                        Err(e) => {
                            self.info.read_failed();
                            return Err(e.into());
                        }
                    };
                }
                Err(e) => {
                    self.info.read_failed();
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
