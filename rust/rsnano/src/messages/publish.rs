use crate::{deserialize_block, utils::Stream, BlockEnum, BlockUniquer, NetworkConstants};
use anyhow::Result;
use std::{
    any::Any,
    fmt::Debug,
    ops::Deref,
    sync::{Arc, RwLock},
};

use super::{Message, MessageHeader, MessageType, MessageVisitor};

#[derive(Clone)]
pub struct Publish {
    header: MessageHeader,
    pub block: Option<Arc<RwLock<BlockEnum>>>, //todo remove Option
    pub digest: u128,
}

impl Publish {
    pub fn new(constants: &NetworkConstants, block: Arc<RwLock<BlockEnum>>) -> Self {
        let mut header = MessageHeader::new(constants, MessageType::Publish);
        header.set_block_type(block.read().unwrap().block_type());

        Self {
            header,
            block: Some(block),
            digest: 0,
        }
    }

    pub fn with_header(header: MessageHeader, digest: u128) -> Self {
        Self {
            header,
            block: None,
            digest,
        }
    }

    pub fn from_stream(
        stream: &mut impl Stream,
        header: MessageHeader,
        digest: u128,
        uniquer: Option<&BlockUniquer>,
    ) -> Result<Self> {
        let mut msg = Self::with_header(header, digest);
        msg.deserialize(stream, uniquer)?;
        Ok(msg)
    }

    pub fn deserialize(
        &mut self,
        stream: &mut impl Stream,
        uniquer: Option<&BlockUniquer>,
    ) -> Result<()> {
        debug_assert!(self.header.message_type() == MessageType::Publish);
        self.block = Some(deserialize_block(
            self.header.block_type(),
            stream,
            uniquer,
        )?);
        Ok(())
    }
}

impl Message for Publish {
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
        self.header().serialize(stream)?;
        let block = self.block.as_ref().ok_or_else(|| anyhow!("no block"))?;
        let lck = block.read().unwrap();
        lck.as_block().serialize(stream)
    }

    fn visit(&self, visitor: &mut dyn MessageVisitor) {
        visitor.publish(self)
    }

    fn clone_box(&self) -> Box<dyn Message> {
        Box::new(self.clone())
    }

    fn message_type(&self) -> MessageType {
        MessageType::Publish
    }
}

impl PartialEq for Publish {
    fn eq(&self, other: &Self) -> bool {
        if self.block.is_some() != other.block.is_some() {
            return false;
        }

        if let Some(b1) = &self.block {
            if let Some(b2) = &other.block {
                let lk1 = b1.read().unwrap();
                let lk2 = b2.read().unwrap();
                if lk1.deref() != lk2.deref() {
                    return false;
                }
            }
        }

        self.header == other.header && self.digest == other.digest
    }
}

impl Debug for Publish {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Publish")
            .field("header", &self.header)
            .field("digest", &self.digest)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{utils::MemoryStream, BlockBuilder, DEV_NETWORK_PARAMS};

    #[test]
    fn serialize() -> Result<()> {
        let block = BlockBuilder::state().build()?;
        let block = Arc::new(RwLock::new(BlockEnum::State(block)));
        let network = &DEV_NETWORK_PARAMS.network;
        let publish1 = Publish::new(network, block);

        let mut stream = MemoryStream::new();
        publish1.serialize(&mut stream)?;

        let header = MessageHeader::from_stream(&mut stream)?;
        let mut publish2 = Publish::with_header(header, 0);
        publish2.deserialize(&mut stream, None)?;
        assert_eq!(publish1, publish2);
        Ok(())
    }
}
