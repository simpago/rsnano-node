use super::{Message, MessageHeader, MessageType};
use crate::{utils::Stream, NetworkConstants};
use anyhow::Result;
use std::any::Any;

#[derive(Clone)]
pub struct TelemetryReq {
    header: MessageHeader,
}

impl TelemetryReq {
    pub fn new(constants: &NetworkConstants) -> Self {
        Self {
            header: MessageHeader::new(constants, MessageType::TelemetryReq),
        }
    }

    pub fn with_header(header: &MessageHeader) -> Self {
        Self {
            header: header.clone(),
        }
    }

    pub fn deserialize(&mut self, _stream: &mut impl Stream) -> Result<()> {
        debug_assert!(self.header.message_type() == MessageType::TelemetryReq);
        Ok(())
    }
}

impl Message for TelemetryReq {
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
}
