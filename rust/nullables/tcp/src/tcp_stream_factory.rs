use super::tcp_stream::TcpStream;
use async_trait::async_trait;
use std::net::{SocketAddr, ToSocketAddrs};

pub struct TcpStreamFactory {
    inner: Box<dyn InternalTcpStreamFactory>,
}

impl TcpStreamFactory {
    pub fn new() -> Self {
        Self {
            inner: Box::new(TcpStreamFactoryWrapper {}),
        }
    }

    pub fn new_null() -> Self {
        Self {
            inner: Box::new(NullTcpStreamFactory {}),
        }
    }

    pub async fn connect<A: ToSocketAddrs>(&self, addr: A) -> tokio::io::Result<TcpStream> {
        self.inner
            .connect(addr.to_socket_addrs().unwrap().next().unwrap())
            .await
    }
}

#[async_trait]
trait InternalTcpStreamFactory: Send + Sync {
    async fn connect(&self, addr: SocketAddr) -> tokio::io::Result<TcpStream>;
}

struct NullTcpStreamFactory {}

#[async_trait]
impl InternalTcpStreamFactory for NullTcpStreamFactory {
    async fn connect(&self, _addr: SocketAddr) -> tokio::io::Result<TcpStream> {
        Err(tokio::io::Error::new(
            std::io::ErrorKind::Other,
            "nulled TcpStreamFactory has no configured connections",
        ))
    }
}

struct TcpStreamFactoryWrapper {}

#[async_trait]
impl InternalTcpStreamFactory for TcpStreamFactoryWrapper {
    async fn connect(&self, addr: SocketAddr) -> tokio::io::Result<TcpStream> {
        let tokio_stream = tokio::net::TcpStream::connect(addr).await?;
        Ok(TcpStream::new(tokio_stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;

    #[tokio::test]
    async fn can_be_nulled() {
        let factory = TcpStreamFactory::new_null();
        match factory.connect("127.0.0.1:42").await {
            Ok(_) => panic!("connect should fail"),
            Err(e) => {
                assert_eq!(e.kind(), ErrorKind::Other);
                assert_eq!(
                    e.to_string(),
                    "nulled TcpStreamFactory has no configured connections"
                );
            }
        }
    }
}
