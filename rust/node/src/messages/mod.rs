mod message;
pub use message::*;

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
pub use telemetry_ack::*;

mod asc_pull_req;
pub use asc_pull_req::*;

mod asc_pull_ack;
pub use asc_pull_ack::*;

pub trait MessageVisitor {
    fn received(&mut self, message: &Message);
}

#[cfg(test)]
pub(crate) fn assert_deserializable(original: &Message) {
    use rsnano_core::utils::StreamAdapter;

    let mut serializer = MessageSerializer::default();
    let serialized = serializer.serialize(original);
    let mut stream = StreamAdapter::new(serialized);
    let header = MessageHeader::deserialize(&mut stream).unwrap();
    assert_eq!(
        header.payload_length(),
        serialized.len() - MessageHeader::SERIALIZED_SIZE
    );
    let message_out = Message::deserialize(stream.remaining(), &header, 0).unwrap();
    assert_eq!(message_out, *original);
}