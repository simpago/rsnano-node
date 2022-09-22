use crate::{
    utils::{Deserialize, Serialize, Stream},
    BlockHash, HashOrAccount, NetworkConstants,
};
use anyhow::Result;
use std::{any::Any, mem::size_of};

use super::{Message, MessageHeader, MessageType, MessageVisitor};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BulkPull {
    header: MessageHeader,
    pub start: HashOrAccount,
    pub end: BlockHash,
    pub count: u32,
}

impl BulkPull {
    const COUNT_PRESENT_FLAG: usize = 0;
    const ASCENDING_FLAG: usize = 1;
    pub const EXTENDED_PARAMETERS_SIZE: usize = 8;

    pub fn new(constants: &NetworkConstants) -> Self {
        Self {
            header: MessageHeader::new(constants, MessageType::BulkPull),
            start: HashOrAccount::new(),
            end: BlockHash::new(),
            count: 0,
        }
    }

    pub fn with_header(header: MessageHeader) -> Self {
        Self {
            header,
            start: HashOrAccount::new(),
            end: BlockHash::new(),
            count: 0,
        }
    }

    pub fn from_stream(stream: &mut impl Stream, header: MessageHeader) -> Result<Self> {
        let mut msg = Self::with_header(header);
        msg.deserialize(stream)?;
        Ok(msg)
    }

    fn clone_box(&self) -> Box<dyn Message> {
        Box::new(self.clone())
    }

    pub fn serialized_size(header: &MessageHeader) -> usize {
        HashOrAccount::serialized_size()
            + BlockHash::serialized_size()
            + (if BulkPull::is_count_present_in_header(header) {
                BulkPull::EXTENDED_PARAMETERS_SIZE
            } else {
                0
            })
    }

    pub fn is_count_present(&self) -> bool {
        Self::is_count_present_in_header(&self.header)
    }

    pub fn is_count_present_in_header(header: &MessageHeader) -> bool {
        header.test_extension(Self::COUNT_PRESENT_FLAG)
    }

    pub fn set_count_present(&mut self, present: bool) {
        self.header.set_extension(Self::COUNT_PRESENT_FLAG, present);
    }

    pub fn is_ascending(&self) -> bool {
        self.header.test_extension(Self::ASCENDING_FLAG)
    }

    pub fn set_ascending(&mut self) {
        self.header.set_extension(Self::ASCENDING_FLAG, true);
    }

    pub fn deserialize(&mut self, stream: &mut impl Stream) -> Result<()> {
        debug_assert!(self.header.message_type() == MessageType::BulkPull);

        self.start = HashOrAccount::deserialize(stream)?;
        self.end = BlockHash::deserialize(stream)?;

        if self.is_count_present() {
            let mut extended_parameters_buffers = [0u8; Self::EXTENDED_PARAMETERS_SIZE];
            const_assert!(size_of::<u32>() < (BulkPull::EXTENDED_PARAMETERS_SIZE - 1)); // "count must fit within buffer")

            stream.read_bytes(
                &mut extended_parameters_buffers,
                Self::EXTENDED_PARAMETERS_SIZE,
            )?;
            if extended_parameters_buffers[0] != 0 {
                bail!("extended parameters front was not 0");
            } else {
                self.count =
                    u32::from_le_bytes(extended_parameters_buffers[1..5].try_into().unwrap());
            }
        } else {
            self.count = 0;
        }
        Ok(())
    }
}

impl Message for BulkPull {
    fn header(&self) -> &MessageHeader {
        &self.header
    }

    fn set_header(&mut self, header: &MessageHeader) {
        self.header = header.clone();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn serialize(&self, stream: &mut dyn Stream) -> Result<()> {
        // Ensure the "count_present" flag is set if there
        // is a limit specifed.  Additionally, do not allow
        // the "count_present" flag with a value of 0, since
        // that is a sentinel which we use to mean "all blocks"
        // and that is the behavior of not having the flag set
        // so it is wasteful to do this.
        debug_assert!(
            (self.count == 0 && !self.is_count_present())
                || (self.count != 0 && self.is_count_present())
        );

        self.header.serialize(stream)?;
        self.start.serialize(stream)?;
        self.end.serialize(stream)?;

        if self.is_count_present() {
            let mut count_buffer = [0u8; Self::EXTENDED_PARAMETERS_SIZE];
            const_assert!(size_of::<u32>() < (BulkPull::EXTENDED_PARAMETERS_SIZE - 1)); // count must fit within buffer

            count_buffer[1..5].copy_from_slice(&self.count.to_le_bytes());
            stream.write_bytes(&count_buffer)?;
        }
        Ok(())
    }

    fn visit(&self, visitor: &mut dyn MessageVisitor) {
        visitor.bulk_pull(self)
    }

    fn clone_box(&self) -> Box<dyn Message> {
        Box::new(self.clone())
    }

    fn message_type(&self) -> MessageType {
        MessageType::BulkPull
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{utils::MemoryStream, DEV_NETWORK_PARAMS};

    #[test]
    fn bulk_pull_serialization() -> Result<()> {
        let mut message_in = BulkPull::new(&DEV_NETWORK_PARAMS.network);
        message_in.header.set_flag(BulkPull::ASCENDING_FLAG as u8);
        let mut stream = MemoryStream::new();
        message_in.serialize(&mut stream)?;
        let header = MessageHeader::from_stream(&mut stream)?;
        let message_out = BulkPull::from_stream(&mut stream, header)?;
        assert_eq!(message_in, message_out);
        assert!(message_out.is_ascending());
        Ok(())
    }
}
