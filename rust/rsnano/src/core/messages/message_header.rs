use crate::{
    config::NetworkConstants,
    core::{serialized_block_size, BlockType, Networks},
    utils::Stream,
};
use anyhow::Result;
use bitvec::prelude::*;
use num_traits::FromPrimitive;
use std::{
    fmt::{Debug, Display},
    mem::size_of,
};

use super::{
    AscPullAck, AscPullReq, BulkPull, BulkPullAccount, ConfirmAck, ConfirmReq, FrontierReq,
    Keepalive, NodeIdHandshake, TelemetryAck,
};

/// Message types are serialized to the network and existing values must thus never change as
/// types are added, removed and reordered in the enum.
#[repr(u8)]
#[derive(FromPrimitive, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Invalid = 0x0,
    NotAType = 0x1,
    Keepalive = 0x2,
    Publish = 0x3,
    ConfirmReq = 0x4,
    ConfirmAck = 0x5,
    BulkPull = 0x6,
    BulkPush = 0x7,
    FrontierReq = 0x8,
    /* deleted 0x9 */
    NodeIdHandshake = 0x0a,
    BulkPullAccount = 0x0b,
    TelemetryReq = 0x0c,
    TelemetryAck = 0x0d,
    AscPullReq = 0x0e,
    AscPullAck = 0x0f,
}

impl MessageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageType::Invalid => "invalid",
            MessageType::NotAType => "not_a_type",
            MessageType::Keepalive => "keepalive",
            MessageType::Publish => "publish",
            MessageType::ConfirmReq => "confirm_req",
            MessageType::ConfirmAck => "confirm_ack",
            MessageType::BulkPull => "bulk_pull",
            MessageType::BulkPush => "bulk_push",
            MessageType::FrontierReq => "frontier_req",
            MessageType::NodeIdHandshake => "node_id_handshake",
            MessageType::BulkPullAccount => "bulk_pull_account",
            MessageType::TelemetryReq => "telemetry_req",
            MessageType::TelemetryAck => "telemetry_ack",
            MessageType::AscPullReq => "asc_pull_req",
            MessageType::AscPullAck => "asc_pull_ack",
        }
    }
}

impl Debug for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

const BLOCK_TYPE_MASK: u16 = 0x0f00;
const COUNT_MASK: u16 = 0xf000;

#[derive(Clone, PartialEq, Eq)]
pub struct MessageHeader {
    message_type: MessageType,
    version_using: u8,
    version_max: u8,
    version_min: u8,
    network: Networks,
    extensions: BitArray<u16>,
}

impl MessageHeader {
    pub fn new(constants: &NetworkConstants, message_type: MessageType) -> Self {
        let version_using = constants.protocol_version;
        Self::with_version_using(constants, message_type, version_using)
    }

    pub fn empty() -> Self {
        Self {
            message_type: MessageType::Invalid,
            version_using: 0,
            version_max: 0,
            version_min: 0,
            network: Networks::NanoDevNetwork,
            extensions: BitArray::ZERO,
        }
    }

    pub fn from_stream(stream: &mut impl Stream) -> Result<MessageHeader> {
        let mut header = Self::empty();
        header.deserialize(stream)?;
        Ok(header)
    }

    pub fn with_version_using(
        constants: &NetworkConstants,
        message_type: MessageType,
        version_using: u8,
    ) -> Self {
        Self {
            message_type,
            version_using,
            version_max: constants.protocol_version,
            version_min: constants.protocol_version_min,
            network: constants.current_network,
            extensions: BitArray::ZERO,
        }
    }

    pub fn version_using(&self) -> u8 {
        self.version_using
    }

    pub fn set_version_using(&mut self, version: u8) {
        self.version_using = version;
    }

    pub fn version_max(&self) -> u8 {
        self.version_max
    }

    pub fn version_min(&self) -> u8 {
        self.version_min
    }

    pub fn network(&self) -> Networks {
        self.network
    }

    pub fn set_network(&mut self, network: Networks) {
        self.network = network;
    }

    pub fn message_type(&self) -> MessageType {
        self.message_type
    }

    pub fn extensions(&self) -> u16 {
        self.extensions.data
    }

    pub fn set_extensions(&mut self, value: u16) {
        self.extensions.data = value;
    }

    pub fn test_extension(&self, position: usize) -> bool {
        self.extensions[position]
    }

    pub fn set_extension(&mut self, position: usize, value: bool) {
        self.extensions.set(position, value);
    }

    pub fn set_flag(&mut self, flag: u8) {
        // Flags from 8 are block_type & count
        debug_assert!(flag < 8);
        self.set_extension(flag as usize, true);
    }

    pub fn block_type(&self) -> BlockType {
        let mut value = self.extensions & BitArray::new(BLOCK_TYPE_MASK);
        value.shift_left(8);
        BlockType::from_u16(value.data).unwrap_or(BlockType::Invalid)
    }

    pub fn set_block_type(&mut self, block_type: BlockType) {
        self.extensions &= BitArray::new(!BLOCK_TYPE_MASK);
        self.extensions |= BitArray::new((block_type as u16) << 8);
    }

    pub fn count(&self) -> u8 {
        let mut value = self.extensions & BitArray::new(COUNT_MASK);
        value.shift_left(12);
        value.data as u8
    }

    pub fn set_count(&mut self, count: u8) {
        debug_assert!(count < 16);
        self.extensions &= BitArray::new(!COUNT_MASK);
        self.extensions |= BitArray::new((count as u16) << 12)
    }

    pub fn serialized_size() -> usize {
        size_of::<u8>() // version_using
        + size_of::<u8>() // version_min
        + size_of::<u8>() // version_max
        + size_of::<Networks>()
        + size_of::<MessageType>()
        + size_of::<u16>() // extensions
    }

    pub fn deserialize(&mut self, stream: &mut dyn Stream) -> Result<()> {
        let mut buffer = [0; 2];

        stream.read_bytes(&mut buffer, 2)?;
        self.network = Networks::from_u16(u16::from_be_bytes(buffer))
            .ok_or_else(|| anyhow!("invalid network"))?;

        self.version_max = stream.read_u8()?;
        self.version_using = stream.read_u8()?;
        self.version_min = stream.read_u8()?;
        self.message_type = MessageType::from_u8(stream.read_u8()?)
            .ok_or_else(|| anyhow!("invalid message type"))?;

        stream.read_bytes(&mut buffer, 2)?;
        self.extensions.data = u16::from_le_bytes(buffer);
        Ok(())
    }

    pub fn serialize(&self, stream: &mut dyn Stream) -> Result<()> {
        stream.write_bytes(&(self.network() as u16).to_be_bytes())?;
        stream.write_u8(self.version_max())?;
        stream.write_u8(self.version_using())?;
        stream.write_u8(self.version_min())?;
        stream.write_u8(self.message_type() as u8)?;
        stream.write_bytes(&self.extensions().to_le_bytes())?;
        Ok(())
    }

    const BULK_PULL_COUNT_PRESENT_FLAG: usize = 0;

    pub fn bulk_pull_is_count_present(&self) -> bool {
        self.message_type() == MessageType::BulkPull
            && self.test_extension(Self::BULK_PULL_COUNT_PRESENT_FLAG)
    }

    pub fn payload_length(&self) -> usize {
        match self.message_type {
            MessageType::Keepalive => Keepalive::serialized_size(),
            MessageType::Publish => serialized_block_size(self.block_type()),
            MessageType::ConfirmReq => ConfirmReq::serialized_size(self.block_type(), self.count()),
            MessageType::ConfirmAck => ConfirmAck::serialized_size(self.count()),
            MessageType::BulkPull => BulkPull::serialized_size(self),
            MessageType::BulkPush | MessageType::TelemetryReq => 0,
            MessageType::FrontierReq => FrontierReq::serialized_size(),
            MessageType::NodeIdHandshake => NodeIdHandshake::serialized_size(self),
            MessageType::BulkPullAccount => BulkPullAccount::serialized_size(),
            MessageType::TelemetryAck => TelemetryAck::size_from_header(self),
            MessageType::AscPullReq => AscPullReq::serialized_size(self),
            MessageType::AscPullAck => AscPullAck::serialized_size(self),
            MessageType::Invalid | MessageType::NotAType => {
                debug_assert!(false);
                0
            }
        }
    }

    pub fn is_valid_message_type(&self) -> bool {
        !matches!(
            self.message_type,
            MessageType::Invalid | MessageType::NotAType
        )
    }
}

impl Display for MessageHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "NetID: {:04X}({}), ",
            self.network() as u16,
            self.network().as_str()
        ))?;
        f.write_fmt(format_args!(
            "VerMaxUsingMin: {}/{}/{}, ",
            self.version_max(),
            self.version_using(),
            self.version_min()
        ))?;
        f.write_fmt(format_args!(
            "MsgType: {}({}), ",
            self.message_type() as u8,
            self.message_type().as_str()
        ))?;
        f.write_fmt(format_args!("Extensions: {:04X}", self.extensions()))
    }
}

impl Debug for MessageHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

#[cfg(test)]
mod tests {
    use crate::{utils::MemoryStream, DEV_NETWORK_PARAMS};

    use super::*;

    #[test]
    fn message_header_to_string() {
        assert_eq!(
            test_header().to_string(),
            "NetID: 5241(dev), VerMaxUsingMin: 3/2/1, MsgType: 2(keepalive), Extensions: 000E"
        );
    }

    #[test]
    fn serialize_and_deserialize() -> Result<()> {
        let original = test_header();
        let mut stream = MemoryStream::new();
        original.serialize(&mut stream)?;
        let deserialized = MessageHeader::from_stream(&mut stream)?;
        assert_eq!(original, deserialized);
        Ok(())
    }

    #[test]
    fn block_type() {
        let mut header = test_header();
        assert_eq!(header.block_type(), BlockType::Invalid);
        header.set_block_type(BlockType::Receive);
        assert_eq!(header.block_type(), BlockType::Receive);
    }

    fn test_header() -> MessageHeader {
        let header = MessageHeader {
            message_type: MessageType::Keepalive,
            version_using: 2,
            version_max: 3,
            version_min: 1,
            network: Networks::NanoDevNetwork,
            extensions: BitArray::from(14),
        };
        header
    }

    #[test]
    fn serialize_header() -> Result<()> {
        let network = &DEV_NETWORK_PARAMS.network;
        let mut header = MessageHeader::new(&network, MessageType::Publish);
        header.set_block_type(BlockType::State);

        let mut stream = MemoryStream::new();
        header.serialize(&mut stream)?;

        let bytes = stream.as_bytes();
        assert_eq!(bytes.len(), 8);
        assert_eq!(bytes[0], 0x52);
        assert_eq!(bytes[1], 0x41);
        assert_eq!(bytes[2], network.protocol_version);
        assert_eq!(bytes[3], network.protocol_version);
        assert_eq!(bytes[4], network.protocol_version_min);
        assert_eq!(bytes[5], 0x03); // publish
        assert_eq!(bytes[6], 0x00); // extensions
        assert_eq!(bytes[7], 0x06); // state block
        Ok(())
    }
}
