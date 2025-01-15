use crate::{
    bandwidth_limiter::BandwidthLimiter, Channel, ChannelDirection, ChannelId, NullNetworkObserver,
};
use rsnano_core::utils::{TEST_ENDPOINT_1, TEST_ENDPOINT_2};
use rsnano_nullable_clock::{SteadyClock, Timestamp};
use rsnano_nullable_tcp::TcpStream;
use std::{
    fmt::Display,
    sync::{Arc, Weak},
    time::Duration,
};
use tokio::{select, time::sleep};

/// Connects a Channel with a TcpStream
pub struct TcpChannelAdapter {
    pub channel: Arc<Channel>,
    stream: Weak<TcpStream>,
    clock: Arc<SteadyClock>,
}

impl TcpChannelAdapter {
    fn new(channel: Arc<Channel>, stream: Weak<TcpStream>, clock: Arc<SteadyClock>) -> Self {
        Self {
            channel,
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
            Arc::new(Channel::new(
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
        channel: Arc<Channel>,
        stream: TcpStream,
        clock: Arc<SteadyClock>,
        handle: &tokio::runtime::Handle,
    ) -> Arc<Self> {
        let stream = Arc::new(stream);
        let channel_adapter = Self::new(channel.clone(), Arc::downgrade(&stream), clock.clone());

        // process write queue:
        handle.spawn(async move {
            loop {
                let res = select! {
                    _ = channel.cancelled() =>{
                        return;
                    },
                  res = channel.pop() => res
                };

                if let Some(entry) = res {
                    let mut written = 0;
                    let buffer = &entry.buffer;
                    loop {
                        select! {
                            _ = channel.cancelled() =>{
                                return;
                            }
                            res = stream.writable() =>{
                            match res {
                            Ok(()) => match stream.try_write(&buffer[written..]) {
                                Ok(n) => {
                                    written += n;
                                    if written >= buffer.len() {
                                        channel.set_last_activity(clock.now());
                                        break;
                                    }
                                }
                                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                    continue;
                                }
                                Err(_) => {
                                    channel.close();
                                    return;
                                }
                            },
                            Err(_) => {
                                channel.close();
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
            channel.close();
        });

        let channel = Arc::new(channel_adapter);
        let channel_l = channel.clone();
        handle.spawn(async move { channel_l.ongoing_checkup().await });
        channel
    }

    pub async fn readable(&self) -> anyhow::Result<()> {
        if self.channel.is_closed() {
            return Err(anyhow!("Tried to read from a closed TcpStream"));
        }

        let Some(stream) = self.stream.upgrade() else {
            return Err(anyhow!("TCP stream dropped"));
        };

        let res = select! {
            _  = self.channel.cancelled() =>{
                return Err(anyhow!("cancelled"));
            },
            res = stream.readable() => res
        };

        match res {
            Ok(_) => Ok(()),
            Err(e) => {
                self.channel.read_failed();
                return Err(e.into());
            }
        }
    }

    pub fn try_read(&self, buffer: &mut [u8]) -> anyhow::Result<usize> {
        let Some(stream) = self.stream.upgrade() else {
            return Err(anyhow!("TCP stream dropped"));
        };

        match stream.try_read(buffer) {
            Ok(0) => {
                self.channel.read_failed();
                Err(anyhow!("remote side closed the channel"))
            }
            Ok(n) => {
                self.channel.read_succeeded(n, self.clock.now());
                Ok(n)
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(0),
            Err(e) => {
                self.channel.read_failed();
                Err(e.into())
            }
        }
    }

    async fn ongoing_checkup(&self) {
        loop {
            sleep(Duration::from_secs(2)).await;
            let now = self.clock.now();
            let timed_out = self.channel.check_timeout(now);
            if timed_out {
                break;
            }
        }
    }
}

impl Display for TcpChannelAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.channel.peer_addr().fmt(f)
    }
}

impl Drop for TcpChannelAdapter {
    fn drop(&mut self) {
        self.channel.close();
    }
}
