use async_trait::async_trait;
use rsnano_network::AsyncBufferReader;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct VecBufferReader {
    buffer: Vec<u8>,
    position: AtomicUsize,
}

impl VecBufferReader {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self {
            buffer,
            position: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl AsyncBufferReader for VecBufferReader {
    async fn read(&self, buffer: &mut [u8], count: usize) -> anyhow::Result<()> {
        let pos = self.position.load(Ordering::SeqCst);
        if count > self.buffer.len() - pos {
            bail!("no more data to read");
        }
        buffer[..count].copy_from_slice(&self.buffer[pos..pos + count]);
        self.position.store(pos + count, Ordering::SeqCst);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_vec() {
        let reader = VecBufferReader::new(Vec::new());
        let mut buffer = vec![0u8; 3];
        let result = reader.read(&mut buffer, 1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn read_one_byte() {
        let reader = VecBufferReader::new(vec![42]);
        let mut buffer = vec![0u8; 1];
        let result = reader.read(&mut buffer, 1).await;
        assert!(result.is_ok());
        assert_eq!(buffer[0], 42);
    }

    #[tokio::test]
    async fn multiple_reads() {
        let reader = VecBufferReader::new(vec![1, 2, 3, 4, 5]);
        let mut buffer = vec![0u8; 2];
        reader.read(&mut buffer, 1).await.unwrap();
        assert_eq!(buffer[0], 1);

        reader.read(&mut buffer, 2).await.unwrap();
        assert_eq!(buffer[0], 2);
        assert_eq!(buffer[1], 3);

        reader.read(&mut buffer, 2).await.unwrap();
        assert_eq!(buffer[0], 4);
        assert_eq!(buffer[1], 5);

        assert!(reader.read(&mut buffer, 1).await.is_err());
    }
}
