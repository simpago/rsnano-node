use crate::{
    utils::{Deserialize, Serialize, Stream},
    Account, NetworkConstants,
};
use anyhow::Result;
use std::{any::Any, mem::size_of};

use super::{Message, MessageHeader, MessageType, MessageVisitor};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrontierReq {
    header: MessageHeader,
    pub start: Account,
    pub age: u32,
    pub count: u32,
}

impl FrontierReq {
    pub fn new(constants: &NetworkConstants) -> Self {
        Self {
            header: MessageHeader::new(constants, MessageType::FrontierReq),
            start: *Account::zero(),
            age: 0,
            count: 0,
        }
    }

    pub fn with_header(header: MessageHeader) -> Self {
        Self {
            header,
            start: *Account::zero(),
            age: 0,
            count: 0,
        }
    }

    pub fn from_stream(stream: &mut impl Stream, header: MessageHeader) -> Result<Self> {
        let mut msg = Self::with_header(header);
        msg.deserialize(stream)?;
        Ok(msg)
    }

    pub fn serialized_size() -> usize {
        Account::serialized_size()
        + size_of::<u32>() // age
        + size_of::<u32>() //count
    }

    pub fn deserialize(&mut self, stream: &mut impl Stream) -> Result<()> {
        debug_assert!(self.header.message_type() == MessageType::FrontierReq);
        self.start = Account::deserialize(stream)?;
        let mut buffer = [0u8; 4];
        stream.read_bytes(&mut buffer, 4)?;
        self.age = u32::from_le_bytes(buffer);
        stream.read_bytes(&mut buffer, 4)?;
        self.count = u32::from_le_bytes(buffer);
        Ok(())
    }

    pub fn is_confirmed_present(&self) -> bool {
        self.header.test_extension(Self::ONLY_CONFIRMED)
    }

    pub const ONLY_CONFIRMED: usize = 1;
}

impl Message for FrontierReq {
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
        self.header.serialize(stream)?;
        self.start.serialize(stream)?;
        stream.write_bytes(&self.age.to_le_bytes())?;
        stream.write_bytes(&self.count.to_le_bytes())
    }

    fn visit(&self, visitor: &mut dyn MessageVisitor) {
        visitor.frontier_req(self)
    }

    fn clone_box(&self) -> Box<dyn Message> {
        Box::new(self.clone())
    }

    fn message_type(&self) -> MessageType {
        MessageType::FrontierReq
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{utils::MemoryStream, NetworkConstants};

    #[test]
    fn serialize() -> Result<()> {
        let constants = NetworkConstants::empty();
        let mut request1 = FrontierReq::new(&constants);
        request1.start = Account::from(1);
        request1.age = 2;
        request1.count = 3;
        let mut stream = MemoryStream::new();
        request1.serialize(&mut stream)?;

        let header = MessageHeader::from_stream(&mut stream)?;
        let mut request2 = FrontierReq::with_header(header);
        request2.deserialize(&mut stream)?;

        assert_eq!(request1, request2);
        Ok(())
    }
}
