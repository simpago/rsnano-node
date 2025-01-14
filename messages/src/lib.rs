#[macro_use]
extern crate num_derive;

#[macro_use]
extern crate anyhow;

#[macro_use]
extern crate static_assertions;

mod message;
mod message_deserializer;
mod network_filter;

pub use message::*;
pub use message_deserializer::*;
pub use network_filter::*;

mod message_serializer;
pub use message_serializer::*;

mod message_header;
pub use message_header::*;

mod node_id_handshake;
pub use node_id_handshake::*;

mod keepalive;
pub use keepalive::*;

mod publish;
pub use publish::*;

mod confirm_req;
pub use confirm_req::*;

mod confirm_ack;
pub use confirm_ack::*;

mod frontier_req;
pub use frontier_req::*;

mod bulk_pull;
pub use bulk_pull::*;

mod bulk_pull_account;
pub use bulk_pull_account::*;

mod telemetry_ack;
use rsnano_core::utils::BufferReader;
pub use telemetry_ack::*;

mod asc_pull_req;
pub use asc_pull_req::*;

mod asc_pull_ack;
pub use asc_pull_ack::*;

pub trait MessageVisitor {
    fn received(&mut self, message: &Message);
}

pub type Cookie = [u8; 32];

pub fn deserialize_message(buffer: &[u8]) -> anyhow::Result<(MessageHeader, Message)> {
    let (header_bytes, payload_bytes) = buffer.split_at(MessageHeader::SERIALIZED_SIZE);
    let header = MessageHeader::deserialize_slice(header_bytes)?;
    let message = Message::deserialize(payload_bytes, &header, 0)
        .ok_or_else(|| anyhow!("invalid message payload"))?;
    Ok((header, message))
}

#[cfg(test)]
pub(crate) fn assert_deserializable(original: &Message) {
    let mut serializer = MessageSerializer::default();
    let serialized = serializer.serialize(original);
    let mut buffer = BufferReader::new(serialized);
    let header = MessageHeader::deserialize(&mut buffer).unwrap();
    assert_eq!(
        header.payload_length(),
        serialized.len() - MessageHeader::SERIALIZED_SIZE,
        "serialized message has incorrect payload length"
    );
    let message_out = Message::deserialize(buffer.remaining(), &header, 0).unwrap();
    assert_eq!(message_out, *original);
}
