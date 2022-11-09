use crate::{
    config::NetworkConstants,
    core::HashOrAccount,
    utils::{Deserialize, MemoryStream, Stream, StreamExt},
};
use num_traits::FromPrimitive;
use std::{any::Any, mem::size_of};

use super::{Message, MessageHeader, MessageType, MessageVisitor};

/**
 * Type of requested asc pull data
 * - blocks:
 * - account_info:
 */
#[repr(u8)]
#[derive(Clone, FromPrimitive)]
pub enum AscPullPayloadId {
    Invalid = 0x0,
    Blocks = 0x1,
    AccountInfo = 0x2,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AscPullReqPayload {
    Invalid,
    Blocks(BlocksReqPayload),
    AccountInfo(AccountInfoReqPayload),
}

#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub struct BlocksReqPayload {
    pub start: HashOrAccount,
    pub count: u8,
}

impl BlocksReqPayload {
    fn serialize(&self, stream: &mut dyn Stream) -> anyhow::Result<()> {
        stream.write_bytes(self.start.as_bytes())?;
        stream.write_u8(self.count)?;
        Ok(())
    }

    fn deserialize(&mut self, stream: &mut dyn Stream) -> anyhow::Result<()> {
        self.start = HashOrAccount::deserialize(stream)?;
        self.count = stream.read_u8()?;
        Ok(())
    }
}

#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub struct AccountInfoReqPayload {
    pub target: HashOrAccount,
}

impl AccountInfoReqPayload {
    fn serialize(&self, stream: &mut dyn Stream) -> anyhow::Result<()> {
        stream.write_bytes(self.target.as_bytes())
    }

    fn deserialize(&mut self, stream: &mut dyn Stream) -> anyhow::Result<()> {
        self.target = HashOrAccount::deserialize(stream)?;
        Ok(())
    }
}

/// Ascending bootstrap pull request
#[derive(Clone)]
pub struct AscPullReq {
    header: MessageHeader,
    payload: AscPullReqPayload,
    pub id: u64,
}

impl AscPullReq {
    pub fn new(constants: &NetworkConstants) -> Self {
        Self {
            header: MessageHeader::new(constants, MessageType::AscPullReq),
            payload: AscPullReqPayload::Invalid,
            id: 0,
        }
    }

    pub fn with_header(header: MessageHeader) -> Self {
        Self {
            header,
            payload: AscPullReqPayload::Invalid,
            id: 0,
        }
    }

    pub fn from_stream(stream: &mut impl Stream, header: MessageHeader) -> anyhow::Result<Self> {
        let mut msg = Self::with_header(header);
        msg.deserialize(stream)?;
        Ok(msg)
    }

    pub fn payload(&self) -> &AscPullReqPayload {
        &self.payload
    }

    pub fn deserialize(&mut self, stream: &mut impl Stream) -> anyhow::Result<()> {
        debug_assert!(self.header.message_type() == MessageType::AscPullReq);
        let pull_type =
            AscPullPayloadId::from_u8(stream.read_u8()?).unwrap_or(AscPullPayloadId::Invalid);
        self.id = stream.read_u64_be()?;

        self.payload = match pull_type {
            AscPullPayloadId::Blocks => {
                let mut payload = BlocksReqPayload::default();
                payload.deserialize(stream)?;
                AscPullReqPayload::Blocks(payload)
            }
            AscPullPayloadId::AccountInfo => {
                let mut payload = AccountInfoReqPayload::default();
                payload.deserialize(stream)?;
                AscPullReqPayload::AccountInfo(payload)
            }
            AscPullPayloadId::Invalid => bail!("Unknown asc_pull_type"),
        };
        Ok(())
    }

    pub fn serialized_size(header: &MessageHeader) -> usize {
        let payload_len = header.extensions() as usize;
        Self::partial_size() + payload_len
    }

    /**
     * Update payload size stored in header
     * IMPORTANT: Must be called after any update to the payload
     */
    fn update_header(&mut self) -> anyhow::Result<()> {
        let mut stream = MemoryStream::new();
        self.serialize_payload(&mut stream)?;
        let payload_len: u16 = stream.as_bytes().len().try_into()?;
        self.header.set_extensions(payload_len);
        Ok(())
    }

    fn serialize_payload(&self, stream: &mut dyn Stream) -> anyhow::Result<()> {
        match &self.payload {
            AscPullReqPayload::Invalid => bail!("invalid payload"),
            AscPullReqPayload::Blocks(blocks) => blocks.serialize(stream),
            AscPullReqPayload::AccountInfo(account_info) => account_info.serialize(stream),
        }
    }

    pub fn payload_type(&self) -> AscPullPayloadId {
        match self.payload {
            AscPullReqPayload::Invalid => AscPullPayloadId::Invalid,
            AscPullReqPayload::Blocks(_) => AscPullPayloadId::Blocks,
            AscPullReqPayload::AccountInfo(_) => AscPullPayloadId::AccountInfo,
        }
    }

    /** Size of message without payload */
    const fn partial_size() -> usize {
        size_of::<u8>() // pull type
        + size_of::<u64>() // id
    }

    pub fn request_blocks(&mut self, payload: BlocksReqPayload) -> anyhow::Result<()> {
        self.payload = AscPullReqPayload::Blocks(payload);
        self.update_header()
    }

    pub fn request_account_info(&mut self, payload: AccountInfoReqPayload) -> anyhow::Result<()> {
        self.payload = AscPullReqPayload::AccountInfo(payload);
        self.update_header()
    }

    pub fn request_invalid(&mut self) {
        self.payload = AscPullReqPayload::Invalid;
        self.header.set_extensions(0);
    }
}

impl Message for AscPullReq {
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

    fn serialize(&self, stream: &mut dyn Stream) -> anyhow::Result<()> {
        self.header.serialize(stream)?;
        stream.write_u8(self.payload_type() as u8)?;
        stream.write_u64_be(self.id)?;
        self.serialize_payload(stream)
    }

    fn visit(&self, visitor: &mut dyn MessageVisitor) {
        visitor.asc_pull_req(self);
    }

    fn clone_box(&self) -> Box<dyn Message> {
        Box::new(self.clone())
    }

    fn message_type(&self) -> MessageType {
        MessageType::AscPullReq
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DEV_NETWORK_PARAMS;

    #[test]
    fn serialize_header() -> anyhow::Result<()> {
        let mut original = AscPullReq::new(&DEV_NETWORK_PARAMS.network);
        original.request_blocks(BlocksReqPayload {
            start: HashOrAccount::from(3),
            count: 111,
        })?;

        let mut stream = MemoryStream::new();
        original.serialize(&mut stream)?;

        let header = MessageHeader::from_stream(&mut stream)?;
        assert_eq!(header.message_type(), MessageType::AscPullReq);
        Ok(())
    }

    #[test]
    fn missing_payload() {
        let original = AscPullReq::new(&DEV_NETWORK_PARAMS.network);
        let mut stream = MemoryStream::new();
        let result = original.serialize(&mut stream);
        match result {
            Ok(_) => panic!("serialize should fail"),
            Err(e) => assert_eq!(e.to_string(), "invalid payload"),
        }
    }

    #[test]
    fn serialize_blocks() -> anyhow::Result<()> {
        let mut original = AscPullReq::new(&DEV_NETWORK_PARAMS.network);
        original.id = 7;
        original.request_blocks(BlocksReqPayload {
            start: HashOrAccount::from(3),
            count: 111,
        })?;

        let mut stream = MemoryStream::new();
        original.serialize(&mut stream)?;

        let header = MessageHeader::from_stream(&mut stream)?;
        let message_out = AscPullReq::from_stream(&mut stream, header)?;
        assert_eq!(message_out.id, original.id);
        assert_eq!(message_out.payload(), original.payload());
        assert!(stream.at_end());
        Ok(())
    }

    #[test]
    fn serialize_account_info() -> anyhow::Result<()> {
        let mut original = AscPullReq::new(&DEV_NETWORK_PARAMS.network);
        original.id = 7;
        original.request_account_info(AccountInfoReqPayload {
            target: HashOrAccount::from(123),
        })?;

        let mut stream = MemoryStream::new();
        original.serialize(&mut stream)?;

        let header = MessageHeader::from_stream(&mut stream)?;
        let message_out = AscPullReq::from_stream(&mut stream, header)?;
        assert_eq!(message_out.id, original.id);
        assert_eq!(message_out.payload(), original.payload());
        assert!(stream.at_end());
        Ok(())
    }
}
