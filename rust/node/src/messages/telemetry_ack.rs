use crate::config::NetworkConstants;
use anyhow::Result;
use rsnano_core::utils::{Deserialize, MemoryStream, Serialize, Stream, StreamExt};
use rsnano_core::{
    sign_message, to_hex_string, validate_message, Account, BlockHash, KeyPair, Signature,
};
use serde_derive::Serialize;
use std::any::Any;
use std::fmt::Display;
use std::mem::size_of;
use std::time::{Duration, SystemTime};

use super::{Message, MessageHeader, MessageType, MessageVisitor};

#[repr(u8)]
#[derive(FromPrimitive, Copy, Clone, PartialEq, Eq)]
pub enum TelemetryMaker {
    NfNode = 0,
    NfPrunedNode = 1,
}

#[derive(Clone)]
pub struct TelemetryData {
    pub signature: Signature,
    pub node_id: Account,
    pub block_count: u64,
    pub cemented_count: u64,
    pub unchecked_count: u64,
    pub account_count: u64,
    pub bandwidth_cap: u64,
    pub uptime: u64,
    pub peer_count: u32,
    pub protocol_version: u8,
    pub genesis_block: BlockHash,
    pub major_version: u8,
    pub minor_version: u8,
    pub patch_version: u8,
    pub pre_release_version: u8,
    pub maker: u8, // Where this telemetry information originated
    pub timestamp: SystemTime,
    pub active_difficulty: u64,
    pub unknown_data: Vec<u8>,
}

impl TelemetryData {
    pub fn new() -> Self {
        Self {
            signature: Signature::new(),
            node_id: Account::zero(),
            block_count: 0,
            cemented_count: 0,
            unchecked_count: 0,
            account_count: 0,
            bandwidth_cap: 0,
            uptime: 0,
            peer_count: 0,
            protocol_version: 0,
            genesis_block: BlockHash::zero(),
            major_version: 0,
            minor_version: 0,
            patch_version: 0,
            pre_release_version: 0,
            maker: TelemetryMaker::NfNode as u8,
            timestamp: SystemTime::UNIX_EPOCH,
            active_difficulty: 0,
            unknown_data: Vec::new(),
        }
    }

    /// Size does not include unknown_data
    pub fn serialized_size_of_known_data() -> usize {
        Signature::serialized_size()
        + Account::serialized_size()
        + size_of::<u64>() //block_count
          + size_of::<u64>()// cemented_count 
          + size_of::<u64>() // unchecked_count 
          + size_of::<u64>() // account_count 
          + size_of::<u64>() // bandwidth_cap 
          + size_of::<u32>() // peer_count
          + size_of::<u8>() // protocol_version
          + size_of::<u64>() // uptime 
          + BlockHash::serialized_size()
          + size_of::<u8>() // major_version 
          + size_of::<u8>() // minor_version 
          + size_of::<u8>() // patch_version 
          + size_of::<u8>() // pre_release_version 
          + size_of::<u8>() // maker 
          + size_of::<u64>() // timestamp 
          + size_of::<u64>() //active_difficulty)
    }

    /// This needs to be updated for each new telemetry version
    pub fn latest_size() -> usize {
        Self::serialized_size_of_known_data()
    }

    pub fn serialize(&self, stream: &mut dyn Stream) -> Result<()> {
        self.signature.serialize(stream)?;
        self.serialize_without_signature(stream)
    }

    fn serialize_without_signature(&self, stream: &mut dyn Stream) -> Result<()> {
        // All values should be serialized in big endian
        self.node_id.serialize(stream)?;
        stream.write_u64_be(self.block_count)?;
        stream.write_u64_be(self.cemented_count)?;
        stream.write_u64_be(self.unchecked_count)?;
        stream.write_u64_be(self.account_count)?;
        stream.write_u64_be(self.bandwidth_cap)?;
        stream.write_u32_be(self.peer_count)?;
        stream.write_u8(self.protocol_version)?;
        stream.write_u64_be(self.uptime)?;
        self.genesis_block.serialize(stream)?;
        stream.write_u8(self.major_version)?;
        stream.write_u8(self.minor_version)?;
        stream.write_u8(self.patch_version)?;
        stream.write_u8(self.pre_release_version)?;
        stream.write_u8(self.maker)?;
        stream.write_u64_be(
            self.timestamp
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_millis() as u64,
        )?;
        stream.write_u64_be(self.active_difficulty)?;
        stream.write_bytes(&self.unknown_data)?;
        Ok(())
    }

    pub fn deserialize(&mut self, stream: &mut dyn Stream, payload_length: u16) -> Result<()> {
        self.signature = Signature::deserialize(stream)?;
        self.node_id = Account::deserialize(stream)?;
        self.block_count = stream.read_u64_be()?;
        self.cemented_count = stream.read_u64_be()?;
        self.unchecked_count = stream.read_u64_be()?;
        self.account_count = stream.read_u64_be()?;
        self.bandwidth_cap = stream.read_u64_be()?;
        self.peer_count = stream.read_u32_be()?;
        self.protocol_version = stream.read_u8()?;
        self.uptime = stream.read_u64_be()?;
        self.genesis_block = BlockHash::deserialize(stream)?;
        self.major_version = stream.read_u8()?;
        self.minor_version = stream.read_u8()?;
        self.patch_version = stream.read_u8()?;
        self.pre_release_version = stream.read_u8()?;
        self.maker = stream.read_u8()?;

        let timestamp_ms = stream.read_u64_be()?;
        self.timestamp = SystemTime::UNIX_EPOCH + Duration::from_millis(timestamp_ms);
        self.active_difficulty = stream.read_u64_be()?;

        if payload_length as usize > Self::latest_size() {
            let unknown_len = (payload_length as usize) - Self::latest_size();
            self.unknown_data.resize(unknown_len, 0);
            stream.read_bytes(&mut self.unknown_data, unknown_len)?;
        }
        Ok(())
    }

    pub fn sign(&mut self, keys: &KeyPair) -> Result<()> {
        debug_assert!(keys.public_key() == self.node_id.into());
        let mut stream = MemoryStream::new();
        self.serialize_without_signature(&mut stream)?;
        self.signature = sign_message(&keys.private_key(), &keys.public_key(), stream.as_bytes());
        Ok(())
    }

    pub fn validate_signature(&self) -> bool {
        let mut stream = MemoryStream::new();
        if self.serialize_without_signature(&mut stream).is_ok() {
            validate_message(&self.node_id.into(), stream.as_bytes(), &self.signature).is_ok()
        } else {
            false
        }
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        let ignore_identification_metrics = true;
        let json_dto = TelemetryDataJsonDto {
            block_count: self.block_count.to_string(),
            cemented_count: self.cemented_count.to_string(),
            unchecked_count: self.unchecked_count.to_string(),
            account_count: self.account_count.to_string(),
            bandwidth_cap: self.bandwidth_cap.to_string(),
            peer_count: self.peer_count.to_string(),
            protocol_version: self.protocol_version.to_string(),
            uptime: self.uptime.to_string(),
            genesis_block: self.genesis_block.to_string(),
            major_version: self.major_version.to_string(),
            minor_version: self.minor_version.to_string(),
            patch_version: self.patch_version.to_string(),
            pre_release_version: self.pre_release_version.to_string(),
            maker: self.maker.to_string(),
            timestamp: self
                .timestamp
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .to_string(),
            active_difficulty: to_hex_string(self.active_difficulty),
            node_id: if !ignore_identification_metrics {
                Some(self.node_id.to_node_id())
            } else {
                None
            },
            signature: if !ignore_identification_metrics {
                Some(self.signature.encode_hex())
            } else {
                None
            },
        };

        serde_json::to_string_pretty(&json_dto)
    }
}

#[derive(Serialize)]
struct TelemetryDataJsonDto {
    pub block_count: String,
    pub cemented_count: String,
    pub unchecked_count: String,
    pub account_count: String,
    pub bandwidth_cap: String,
    pub peer_count: String,
    pub protocol_version: String,
    pub uptime: String,
    pub genesis_block: String,
    pub major_version: String,
    pub minor_version: String,
    pub patch_version: String,
    pub pre_release_version: String,
    pub maker: String,
    pub timestamp: String,
    pub active_difficulty: String,
    // Keep these last for UI purposes:
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl Default for TelemetryData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct TelemetryAck {
    header: MessageHeader,
    pub data: TelemetryData,
}

impl TelemetryAck {
    const SIZE_MASK: u16 = 0x3ff;

    pub fn new(constants: &NetworkConstants, data: TelemetryData) -> Self {
        debug_assert!(
            TelemetryData::serialized_size_of_known_data() + data.unknown_data.len()
                <= TelemetryAck::SIZE_MASK as usize
        ); // Maximum size the mask allows
        let mut header = MessageHeader::new(constants, MessageType::TelemetryAck);
        let mut extensions = header.extensions();
        extensions &= !TelemetryAck::SIZE_MASK;
        extensions |=
            TelemetryData::serialized_size_of_known_data() as u16 + data.unknown_data.len() as u16;
        header.set_extensions(extensions);

        Self { header, data }
    }

    pub fn from_stream(stream: &mut impl Stream, header: MessageHeader) -> Result<Self> {
        let mut msg = TelemetryAck::with_header(header);
        msg.deserialize(stream)?;
        Ok(msg)
    }

    pub fn with_header(header: MessageHeader) -> Self {
        Self {
            header,
            data: TelemetryData::new(),
        }
    }

    pub fn deserialize(&mut self, stream: &mut dyn Stream) -> Result<()> {
        debug_assert!(self.header.message_type() == MessageType::TelemetryAck);
        if !self.is_empty_payload() {
            self.data.deserialize(stream, self.header.extensions())?;
        }

        Ok(())
    }

    pub fn size_from_header(header: &MessageHeader) -> usize {
        (header.extensions() & TelemetryAck::SIZE_MASK) as usize
    }

    pub fn size(&self) -> usize {
        TelemetryAck::size_from_header(&self.header)
    }

    pub fn is_empty_payload(&self) -> bool {
        self.size() == 0
    }
}

impl Message for TelemetryAck {
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
        if !self.is_empty_payload() {
            self.data.serialize(stream)?;
        }
        Ok(())
    }

    fn visit(&self, visitor: &mut dyn MessageVisitor) {
        visitor.telemetry_ack(self)
    }

    fn clone_box(&self) -> Box<dyn Message> {
        Box::new(self.clone())
    }

    fn message_type(&self) -> MessageType {
        MessageType::TelemetryAck
    }
}

impl Display for TelemetryAck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.header.fmt(f)?;
        write!(f, "\n")?;
        if self.is_empty_payload() {
            write!(f, "empty telemetry payload")?;
        } else {
            write!(f, "{}", self.data.to_json().map_err(|_| std::fmt::Error)?)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // original test: telemetry.signatures
    #[test]
    fn sign_telemetry_data() -> Result<()> {
        let keys = KeyPair::new();
        let mut data = test_data(&keys);
        data.sign(&keys)?;
        assert_eq!(data.validate_signature(), true);

        let old_signature = data.signature.clone();
        // Check that the signature is different if changing a piece of data
        data.maker = 2;
        data.sign(&keys)?;
        assert_ne!(old_signature, data.signature);
        Ok(())
    }

    //original test: telemetry.unknown_data
    #[test]
    fn sign_with_unknown_data() -> Result<()> {
        let keys = KeyPair::new();
        let mut data = test_data(&keys);
        data.unknown_data = vec![1];
        data.sign(&keys)?;
        assert_eq!(data.validate_signature(), true);
        Ok(())
    }

    fn test_data(keys: &KeyPair) -> TelemetryData {
        let mut data = TelemetryData::new();
        data.node_id = keys.public_key().into();
        data.major_version = 20;
        data.minor_version = 1;
        data.patch_version = 5;
        data.pre_release_version = 2;
        data.maker = 1;
        data.timestamp = SystemTime::UNIX_EPOCH + Duration::from_millis(100);
        data
    }
}
