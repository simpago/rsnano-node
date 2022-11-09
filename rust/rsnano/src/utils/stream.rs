pub trait Stream {
    fn write_u8(&mut self, value: u8) -> anyhow::Result<()>;
    fn write_bytes(&mut self, bytes: &[u8]) -> anyhow::Result<()>;
    fn read_u8(&mut self) -> anyhow::Result<u8>;
    fn read_bytes(&mut self, buffer: &mut [u8], len: usize) -> anyhow::Result<()>;

    ///  Looking ahead into the stream.
    ///  returns:  The number of characters available.
    ///  If a read position is available, returns the number of characters
    ///  available for reading before the buffer must be refilled.
    ///  Otherwise returns the derived showmanyc().
    fn in_avail(&mut self) -> anyhow::Result<usize>;
}

pub trait StreamExt: Stream {
    fn read_u32_be(&mut self) -> anyhow::Result<u32> {
        let mut buffer = [0u8; 4];
        self.read_bytes(&mut buffer, 4)?;
        Ok(u32::from_be_bytes(buffer))
    }

    fn read_u64_be(&mut self) -> anyhow::Result<u64> {
        let mut buffer = [0u8; 8];
        self.read_bytes(&mut buffer, 8)?;
        Ok(u64::from_be_bytes(buffer))
    }

    fn read_u64_ne(&mut self) -> anyhow::Result<u64> {
        let mut buffer = [0u8; 8];
        self.read_bytes(&mut buffer, 8)?;
        Ok(u64::from_ne_bytes(buffer))
    }

    fn write_u32_be(&mut self, value: u32) -> anyhow::Result<()> {
        self.write_bytes(&value.to_be_bytes())
    }

    fn write_u64_be(&mut self, value: u64) -> anyhow::Result<()> {
        self.write_bytes(&value.to_be_bytes())
    }

    fn write_u64_ne(&mut self, value: u64) -> anyhow::Result<()> {
        self.write_bytes(&value.to_ne_bytes())
    }
}

impl<T: Stream + ?Sized> StreamExt for T {}

#[derive(Default)]
pub struct MemoryStream {
    bytes: Vec<u8>,
    read_index: usize,
}

impl MemoryStream {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn bytes_written(&self) -> usize {
        self.bytes.len()
    }

    pub fn byte_at(&self, i: usize) -> u8 {
        self.bytes[i]
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn to_vec(self) -> Vec<u8> {
        self.bytes
    }

    pub fn at_end(&self) -> bool {
        self.bytes.len() - self.read_index == 0
    }
}

impl Stream for MemoryStream {
    fn write_u8(&mut self, value: u8) -> anyhow::Result<()> {
        self.bytes.push(value);
        Ok(())
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    fn read_u8(&mut self) -> anyhow::Result<u8> {
        if self.read_index >= self.bytes.len() {
            bail!("no more bytes to read")
        }

        let result = self.bytes[self.read_index];
        self.read_index += 1;
        Ok(result)
    }

    fn read_bytes(&mut self, buffer: &mut [u8], len: usize) -> anyhow::Result<()> {
        if self.read_index + len > self.bytes.len() {
            bail!("not enough bytes to read")
        }

        buffer.copy_from_slice(&self.bytes[self.read_index..self.read_index + len]);
        self.read_index += len;
        Ok(())
    }

    fn in_avail(&mut self) -> anyhow::Result<usize> {
        Ok(self.bytes.len() - self.read_index)
    }
}

pub struct MutStreamAdapter<'a> {
    bytes: &'a mut [u8],
    read_index: usize,
    write_index: usize,
}

impl<'a> MutStreamAdapter<'a> {
    pub fn new(bytes: &'a mut [u8]) -> Self {
        Self {
            bytes,
            read_index: 0,
            write_index: 0,
        }
    }
}

impl<'a> Stream for MutStreamAdapter<'a> {
    fn write_u8(&mut self, value: u8) -> anyhow::Result<()> {
        if self.write_index >= self.bytes.len() {
            bail!("buffer full");
        }
        self.bytes[self.write_index] = value;
        self.write_index += 1;
        Ok(())
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        if self.write_index + bytes.len() > self.bytes.len() {
            bail!("buffer full");
        }
        self.bytes[self.write_index..self.write_index + bytes.len()].copy_from_slice(bytes);
        self.write_index += bytes.len();
        Ok(())
    }

    fn read_u8(&mut self) -> anyhow::Result<u8> {
        if self.read_index >= self.bytes.len() {
            bail!("no more bytes to read")
        }

        let result = self.bytes[self.read_index];
        self.read_index += 1;
        Ok(result)
    }

    fn read_bytes(&mut self, buffer: &mut [u8], len: usize) -> anyhow::Result<()> {
        if self.read_index + len > self.bytes.len() {
            bail!("not enough bytes to read")
        }

        buffer.copy_from_slice(&self.bytes[self.read_index..self.read_index + len]);
        self.read_index += len;
        Ok(())
    }

    fn in_avail(&mut self) -> anyhow::Result<usize> {
        Ok(self.bytes.len() - self.read_index)
    }
}

pub struct StreamAdapter<'a> {
    bytes: &'a [u8],
    read_index: usize,
}

impl<'a> StreamAdapter<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            read_index: 0,
        }
    }
}

impl<'a> Stream for StreamAdapter<'a> {
    fn write_u8(&mut self, _value: u8) -> anyhow::Result<()> {
        bail!("not supported");
    }

    fn write_bytes(&mut self, _bytes: &[u8]) -> anyhow::Result<()> {
        bail!("not supported");
    }

    fn read_u8(&mut self) -> anyhow::Result<u8> {
        if self.read_index >= self.bytes.len() {
            bail!("no more bytes to read")
        }

        let result = self.bytes[self.read_index];
        self.read_index += 1;
        Ok(result)
    }

    fn read_bytes(&mut self, buffer: &mut [u8], len: usize) -> anyhow::Result<()> {
        if self.read_index + len > self.bytes.len() {
            bail!("not enough bytes to read")
        }

        buffer.copy_from_slice(&self.bytes[self.read_index..self.read_index + len]);
        self.read_index += len;
        Ok(())
    }

    fn in_avail(&mut self) -> anyhow::Result<usize> {
        Ok(self.bytes.len() - self.read_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_stream() -> Result<()> {
        let mut stream = MemoryStream::new();
        stream.write_bytes(&[1, 2, 3])?;
        assert_eq!(stream.bytes_written(), 3);

        let mut read_buffer = [0u8; 3];
        stream.read_bytes(&mut read_buffer, 3)?;
        assert_eq!([1, 2, 3], read_buffer);

        assert!(stream.read_bytes(&mut read_buffer, 1).is_err());
        Ok(())
    }
}
