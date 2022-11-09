use crate::{
    config::NetworkConstants,
    core::{
        deserialize_block_enum, serialize_block_enum, Account, BlockEnum, BlockHash, BlockType,
    },
    utils::{Deserialize, MemoryStream, Serialize, Stream, StreamExt},
};
use num_traits::FromPrimitive;
use std::{any::Any, mem::size_of};

use super::{AscPullPayloadId, Message, MessageHeader, MessageType, MessageVisitor};

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct BlocksAckPayload {
    pub blocks: Vec<BlockEnum>,
}

/* Header allows for 16 bit extensions; 65535 bytes / 500 bytes (block size with some future margin) ~ 131 */
const MAX_BLOCKS: usize = 128;

impl BlocksAckPayload {
    pub fn deserialize(&mut self, stream: &mut dyn Stream) -> anyhow::Result<()> {
        while let Ok(current) = deserialize_block_enum(stream) {
            if self.blocks.len() >= MAX_BLOCKS {
                bail!("too many blocks")
            }
            self.blocks.push(current);
        }
        Ok(())
    }

    pub fn serialize(&self, stream: &mut dyn Stream) -> anyhow::Result<()> {
        if self.blocks.len() > MAX_BLOCKS {
            bail!("too many blocks");
        }

        for block in &self.blocks {
            serialize_block_enum(stream, block)?;
        }
        // For convenience, end with null block terminator
        stream.write_u8(BlockType::NotABlock as u8)
    }
}

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct AccountInfoAckPayload {
    pub account: Account,
    pub account_open: BlockHash,
    pub account_head: BlockHash,
    pub account_block_count: u64,
    pub account_conf_frontier: BlockHash,
    pub account_conf_height: u64,
}

impl AccountInfoAckPayload {
    pub fn serialize(&self, stream: &mut dyn Stream) -> anyhow::Result<()> {
        self.account.serialize(stream)?;
        self.account_open.serialize(stream)?;
        self.account_head.serialize(stream)?;
        stream.write_u64_be(self.account_block_count)?;
        self.account_conf_frontier.serialize(stream)?;
        stream.write_u64_be(self.account_conf_height)
    }

    pub fn deserialize(&mut self, stream: &mut dyn Stream) -> anyhow::Result<()> {
        self.account = Account::deserialize(stream)?;
        self.account_open = BlockHash::deserialize(stream)?;
        self.account_head = BlockHash::deserialize(stream)?;
        self.account_block_count = stream.read_u64_be()?;
        self.account_conf_frontier = BlockHash::deserialize(stream)?;
        self.account_conf_height = stream.read_u64_be()?;
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AscPullAckPayload {
    Invalid,
    Blocks(BlocksAckPayload),
    AccountInfo(AccountInfoAckPayload),
}

#[derive(Clone)]
pub struct AscPullAck {
    header: MessageHeader,
    payload: AscPullAckPayload,
    pub id: u64,
}

impl AscPullAck {
    pub fn new(constants: &NetworkConstants) -> Self {
        Self {
            header: MessageHeader::new(constants, MessageType::AscPullAck),
            payload: AscPullAckPayload::Invalid,
            id: 0,
        }
    }

    pub fn with_header(header: MessageHeader) -> Self {
        Self {
            header,
            payload: AscPullAckPayload::Invalid,
            id: 0,
        }
    }

    pub fn from_stream(stream: &mut impl Stream, header: MessageHeader) -> anyhow::Result<Self> {
        let mut msg = Self::with_header(header);
        msg.deserialize(stream)?;
        Ok(msg)
    }

    pub fn payload_type(&self) -> AscPullPayloadId {
        match self.payload {
            AscPullAckPayload::Invalid => AscPullPayloadId::Invalid,
            AscPullAckPayload::Blocks(_) => AscPullPayloadId::Blocks,
            AscPullAckPayload::AccountInfo(_) => AscPullPayloadId::AccountInfo,
        }
    }

    pub fn payload(&self) -> &AscPullAckPayload {
        &&self.payload
    }

    fn serialize_payload(&self, stream: &mut dyn Stream) -> anyhow::Result<()> {
        match &self.payload {
            AscPullAckPayload::Invalid => Err(anyhow!("missing payload")),
            AscPullAckPayload::Blocks(blocks) => blocks.serialize(stream),
            AscPullAckPayload::AccountInfo(account_info) => account_info.serialize(stream),
        }
    }

    pub fn deserialize(&mut self, stream: &mut impl Stream) -> anyhow::Result<()> {
        debug_assert!(self.header.message_type() == MessageType::AscPullAck);
        let pull_type_code =
            AscPullPayloadId::from_u8(stream.read_u8()?).unwrap_or(AscPullPayloadId::Invalid);
        self.id = stream.read_u64_be()?;
        self.deserialize_payload(stream, pull_type_code)
    }

    fn deserialize_payload(
        &mut self,
        stream: &mut impl Stream,
        pull_type_code: AscPullPayloadId,
    ) -> anyhow::Result<()> {
        self.payload = match pull_type_code {
            AscPullPayloadId::Invalid => bail!("Unknown asc_pull_type"),
            AscPullPayloadId::Blocks => {
                let mut payload = BlocksAckPayload::default();
                payload.deserialize(stream)?;
                AscPullAckPayload::Blocks(payload)
            }
            AscPullPayloadId::AccountInfo => {
                let mut payload = AccountInfoAckPayload::default();
                payload.deserialize(stream)?;
                AscPullAckPayload::AccountInfo(payload)
            }
        };
        Ok(())
    }

    /// Size of message without payload
    const PARTIAL_SIZE: usize = size_of::<u8>() // type code 
    + size_of::<u64>(); // id

    pub fn serialized_size(header: &MessageHeader) -> usize {
        let payload_length = header.extensions() as usize;
        Self::PARTIAL_SIZE + payload_length
    }

    fn update_header(&mut self) -> anyhow::Result<()> {
        let mut stream = MemoryStream::new();
        self.serialize_payload(&mut stream)?;
        let payload_len: u16 = stream.as_bytes().len().try_into()?;
        self.header.set_extensions(payload_len);
        Ok(())
    }

    pub fn request_blocks(&mut self, payload: BlocksAckPayload) -> anyhow::Result<()> {
        self.payload = AscPullAckPayload::Blocks(payload);
        self.update_header()
    }

    pub fn request_account_info(&mut self, payload: AccountInfoAckPayload) -> anyhow::Result<()> {
        self.payload = AscPullAckPayload::AccountInfo(payload);
        self.update_header()
    }

    pub fn request_invalid(&mut self) {
        self.payload = AscPullAckPayload::Invalid;
        self.header.set_extensions(0);
    }
}

impl Message for AscPullAck {
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
        if self.payload == AscPullAckPayload::Invalid {
            bail!("invalid payload");
        }

        if self.header.extensions() == 0 {
            bail!("Block payload must have least `not_a_block` terminator");
        }

        self.header.serialize(stream)?;
        stream.write_u8(self.payload_type() as u8)?;
        stream.write_u64_be(self.id)?;
        self.serialize_payload(stream)
    }

    fn visit(&self, visitor: &mut dyn MessageVisitor) {
        visitor.asc_pull_ack(self);
    }

    fn clone_box(&self) -> Box<dyn Message> {
        Box::new(self.clone())
    }

    fn message_type(&self) -> MessageType {
        MessageType::AscPullAck
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::BlockBuilder, DEV_NETWORK_PARAMS};

    #[test]
    fn serialize_header() -> anyhow::Result<()> {
        let mut original = AscPullAck::new(&DEV_NETWORK_PARAMS.network);
        original.request_blocks(BlocksAckPayload { blocks: vec![] })?;

        let mut stream = MemoryStream::new();
        original.serialize(&mut stream)?;

        let header = MessageHeader::from_stream(&mut stream)?;
        assert_eq!(header.message_type(), MessageType::AscPullAck);
        Ok(())
    }

    #[test]
    fn missing_payload() {
        let original = AscPullAck::new(&DEV_NETWORK_PARAMS.network);
        let mut stream = MemoryStream::new();
        let result = original.serialize(&mut stream);
        match result {
            Ok(_) => panic!("serialize should fail"),
            Err(e) => assert_eq!(e.to_string(), "invalid payload"),
        }
    }

    #[test]
    fn serialize_blocks() -> anyhow::Result<()> {
        let mut original = AscPullAck::new(&DEV_NETWORK_PARAMS.network);
        original.id = 7;
        original.request_blocks(BlocksAckPayload {
            blocks: vec![
                BlockEnum::State(BlockBuilder::state().build()?),
                BlockEnum::State(BlockBuilder::state().build()?),
            ],
        })?;

        let mut stream = MemoryStream::new();
        original.serialize(&mut stream)?;

        let header = MessageHeader::from_stream(&mut stream)?;
        let message_out = AscPullAck::from_stream(&mut stream, header)?;
        assert_eq!(message_out.id, original.id);
        assert_eq!(message_out.payload(), original.payload());
        assert!(stream.at_end());
        Ok(())
    }

    #[test]
    fn serialize_account_info() -> anyhow::Result<()> {
        let mut original = AscPullAck::new(&DEV_NETWORK_PARAMS.network);
        original.id = 7;
        original.request_account_info(AccountInfoAckPayload {
            account: Account::from(1),
            account_open: BlockHash::from(2),
            account_head: BlockHash::from(3),
            account_block_count: 4,
            account_conf_frontier: BlockHash::from(5),
            account_conf_height: 6,
        })?;

        let mut stream = MemoryStream::new();
        original.serialize(&mut stream)?;

        let header = MessageHeader::from_stream(&mut stream)?;
        let message_out = AscPullAck::from_stream(&mut stream, header)?;
        assert_eq!(message_out.id, original.id);
        assert_eq!(message_out.payload(), original.payload());
        assert!(stream.at_end());
        Ok(())
    }
}
