use super::{Message, MessageHeader, MessageType, MessageVisitor};
use crate::{utils::Stream, NetworkConstants};
use anyhow::Result;
use std::any::Any;

#[derive(Clone)]
pub struct BulkPush {
    header: MessageHeader,
}

impl BulkPush {
    pub fn new(constants: &NetworkConstants) -> Self {
        Self {
            header: MessageHeader::new(constants, MessageType::BulkPush),
        }
    }

    pub fn with_header(header: MessageHeader) -> Self {
        Self { header }
    }

    pub fn deserialize(&mut self, _stream: &mut impl Stream) -> Result<()> {
        debug_assert!(self.header.message_type() == MessageType::BulkPush);
        Ok(())
    }
}

impl Message for BulkPush {
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
        self.header.serialize(stream)
    }

    fn visit(&self, visitor: &mut dyn MessageVisitor) {
        visitor.bulk_push(self)
    }

    fn clone_box(&self) -> Box<dyn Message> {
        Box::new(self.clone())
    }

    fn message_type(&self) -> MessageType {
        MessageType::BulkPush
    }
}
