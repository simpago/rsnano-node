use std::{
    cmp::min,
    net::{Ipv6Addr, SocketAddr, SocketAddrV6},
    sync::atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use tokio::io::{AsyncWriteExt, ErrorKind};

pub const TEST_ENDPOINT_1: SocketAddrV6 =
    SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0xffff, 0x10, 0, 0, 1), 1111, 0, 0);

pub struct TcpStream {
    stream: Box<dyn InternalTcpStream>,
}

impl TcpStream {
    pub fn new(stream: tokio::net::TcpStream) -> Self {
        Self {
            stream: Box::new(TokioTcpStreamWrapper(stream)),
        }
    }

    pub fn new_null() -> Self {
        Self {
            stream: Box::new(TcpStreamStub::new(TEST_ENDPOINT_1, Vec::new())),
        }
    }

    pub fn new_null_with_peer_addr(peer_addr: SocketAddrV6) -> Self {
        Self {
            stream: Box::new(TcpStreamStub::new(peer_addr, Vec::new())),
        }
    }

    pub fn new_null_with(incoming: Vec<u8>) -> Self {
        Self {
            stream: Box::new(TcpStreamStub::new(TEST_ENDPOINT_1, incoming)),
        }
    }

    pub async fn shutdown(&mut self) -> tokio::io::Result<()> {
        self.stream.shutdown().await
    }

    pub async fn readable(&self) -> tokio::io::Result<()> {
        self.stream.readable().await
    }

    pub fn try_read(&self, buf: &mut [u8]) -> tokio::io::Result<usize> {
        self.stream.try_read(buf)
    }

    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.stream.local_addr()
    }

    pub fn peer_addr(&self) -> std::io::Result<SocketAddr> {
        self.stream.peer_addr()
    }

    pub async fn writable(&self) -> tokio::io::Result<()> {
        self.stream.writable().await
    }

    pub fn try_write(&self, buf: &[u8]) -> tokio::io::Result<usize> {
        self.stream.try_write(buf)
    }
}

#[async_trait]
trait InternalTcpStream: Send + Sync {
    async fn readable(&self) -> tokio::io::Result<()>;
    fn try_read(&self, buf: &mut [u8]) -> tokio::io::Result<usize>;
    fn local_addr(&self) -> std::io::Result<SocketAddr>;
    fn peer_addr(&self) -> std::io::Result<SocketAddr>;
    async fn writable(&self) -> tokio::io::Result<()>;
    fn try_write(&self, buf: &[u8]) -> tokio::io::Result<usize>;
    async fn shutdown(&mut self) -> tokio::io::Result<()>;
}

struct TokioTcpStreamWrapper(tokio::net::TcpStream);

#[async_trait]
impl InternalTcpStream for TokioTcpStreamWrapper {
    async fn readable(&self) -> tokio::io::Result<()> {
        self.0.readable().await
    }

    fn try_read(&self, buf: &mut [u8]) -> tokio::io::Result<usize> {
        self.0.try_read(buf)
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.0.local_addr()
    }

    fn peer_addr(&self) -> std::io::Result<SocketAddr> {
        self.0.peer_addr()
    }

    async fn writable(&self) -> tokio::io::Result<()> {
        self.0.writable().await
    }

    fn try_write(&self, buf: &[u8]) -> tokio::io::Result<usize> {
        self.0.try_write(buf)
    }

    async fn shutdown(&mut self) -> tokio::io::Result<()> {
        self.0.shutdown().await
    }
}

struct TcpStreamStub {
    incoming: Vec<u8>,
    position: AtomicUsize,
    peer_addr: SocketAddrV6,
}

impl TcpStreamStub {
    pub fn new(peer_addr: SocketAddrV6, incoming: Vec<u8>) -> Self {
        Self {
            incoming,
            position: AtomicUsize::new(0),
            peer_addr,
        }
    }

    fn no_data_error() -> tokio::io::Error {
        tokio::io::Error::new(ErrorKind::Other, "nulled tcp stream has no data")
    }

    fn next_bytes(&self) -> &[u8] {
        let pos = self.position.load(Ordering::SeqCst);
        &self.incoming[pos..]
    }
}

#[async_trait]
impl InternalTcpStream for TcpStreamStub {
    async fn readable(&self) -> tokio::io::Result<()> {
        if self.next_bytes().is_empty() {
            Err(Self::no_data_error())
        } else {
            Ok(())
        }
    }

    fn try_read(&self, buf: &mut [u8]) -> tokio::io::Result<usize> {
        let next_bytes = self.next_bytes();
        if next_bytes.is_empty() {
            Err(Self::no_data_error())
        } else {
            let read_count = min(buf.len(), next_bytes.len());
            buf[..read_count].copy_from_slice(&next_bytes[..read_count]);
            self.position.fetch_add(read_count, Ordering::SeqCst);
            Ok(read_count)
        }
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        Ok(SocketAddr::V6(TEST_ENDPOINT_1))
    }

    fn peer_addr(&self) -> std::io::Result<SocketAddr> {
        Ok(SocketAddr::V6(self.peer_addr))
    }

    async fn writable(&self) -> tokio::io::Result<()> {
        Ok(())
    }

    fn try_write(&self, buf: &[u8]) -> tokio::io::Result<usize> {
        Ok(buf.len())
    }

    async fn shutdown(&mut self) -> tokio::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TcpStreamFactory;
    use std::{
        io::ErrorKind,
        net::{IpAddr, Ipv4Addr, SocketAddr},
    };
    use tokio::{net::TcpListener, spawn};

    #[tokio::test]
    async fn connects_to_real_server() {
        let endpoint = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8088);
        start_test_tcp_server(endpoint).await;

        let stream_factory = TcpStreamFactory::new();
        let stream = stream_factory.connect("127.0.0.1:8088").await.unwrap();

        let mut buf = [0; 3];
        loop {
            stream.readable().await.unwrap();
            match stream.try_read(&mut buf) {
                Ok(0) => break,
                Ok(_) => {
                    break;
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    panic!("unexpected error when reading {:?}", e);
                }
            }
        }
        assert_eq!(buf, [1, 2, 3]);
    }

    #[tokio::test]
    async fn nulled_stream_returns_error_when_calling_readable() {
        let stream = TcpStream::new_null();
        let error = stream.readable().await.expect_err("readable should fail");
        assert_eq!(error.kind(), ErrorKind::Other);
        assert_eq!(error.to_string(), "nulled tcp stream has no data");
    }

    #[tokio::test]
    async fn nulled_stream_returns_error_when_calling_try_read() {
        let stream = TcpStream::new_null();
        let error = stream.try_read(&mut [0]).expect_err("try_read should fail");
        assert_eq!(error.kind(), ErrorKind::Other);
        assert_eq!(error.to_string(), "nulled tcp stream has no data");
    }

    #[tokio::test]
    async fn nulled_stream_should_read_configured_data() {
        let stream = TcpStream::new_null_with(vec![1, 2, 3]);
        stream.readable().await.expect("readable should not fail");
        let mut buf = [0; 3];
        let read_count = stream.try_read(&mut buf).expect("try_read should not fail");
        assert_eq!(read_count, 3);
        assert_eq!(buf, [1, 2, 3]);
    }

    #[tokio::test]
    async fn nulled_stream_should_read_configured_data_into_bigger_buffer() {
        let stream = TcpStream::new_null_with(vec![1, 2, 3]);
        stream.readable().await.expect("readable should not fail");
        let mut buf = [0; 5];
        let read_count = stream.try_read(&mut buf).expect("try_read should not fail");
        assert_eq!(read_count, 3);
        assert_eq!(buf, [1, 2, 3, 0, 0]);
    }

    #[tokio::test]
    async fn nulled_stream_can_read_configured_data_with_multiple_reads() {
        let stream = TcpStream::new_null_with(vec![1, 2, 3]);

        //read first chunk
        stream.readable().await.expect("readable should not fail");
        let mut buf = [0; 2];
        let read_count = stream.try_read(&mut buf).expect("try_read should not fail");
        assert_eq!(read_count, 2);
        assert_eq!(buf, [1, 2]);

        //read second chunk
        let mut buf = [0; 2];
        stream.readable().await.expect("readable should not fail");
        let read_count = stream.try_read(&mut buf).expect("try_read should not fail");
        assert_eq!(read_count, 1);
        assert_eq!(buf, [3, 0]);
    }

    #[tokio::test]
    async fn nulled_stream_should_fail_after_all_incoming_data_was_read() {
        let stream = TcpStream::new_null_with(vec![1, 2, 3]);
        stream.readable().await.expect("readable should not fail");
        let mut buf = [0; 5];
        let read_count = stream.try_read(&mut buf).expect("try_read should not fail");
        assert_eq!(read_count, 3);

        stream
            .readable()
            .await
            .expect_err("readable should fail on second call");
        stream
            .try_read(&mut buf)
            .expect_err("try_read should fail on second call");
    }

    async fn start_test_tcp_server(endpoint: SocketAddr) {
        let listener = TcpListener::bind(endpoint).await.unwrap();

        spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            loop {
                socket.writable().await.unwrap();
                match socket.try_write(&[1, 2, 3]) {
                    Ok(_) => {
                        break;
                    }
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                        continue;
                    }
                    Err(e) => {
                        panic!("unexpected error: {:?}", e);
                    }
                }
            }
        });
    }
}
