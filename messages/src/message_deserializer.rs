use crate::{DeserializedMessage, Message, MessageHeader, ParseMessageError};
use std::{collections::VecDeque, io::Read};

pub struct MessageDeserializerV2 {
    buffer: VecDeque<u8>,
    current_header: Option<MessageHeader>,
}

impl MessageDeserializerV2 {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::with_capacity(Message::MAX_MESSAGE_SIZE),
            current_header: None,
        }
    }

    pub fn push(&mut self, data: &[u8]) {
        self.buffer.extend(data);
    }

    pub fn try_deserialize(&mut self) -> Option<Result<DeserializedMessage, ParseMessageError>> {
        if let Some(header) = self.current_header.clone() {
            // The header was already deserialized. Only the payload has to be read:
            self.deserialize_payload_for(header)
        } else {
            self.deserialize_header_and_payload()
        }
    }

    fn deserialize_header_and_payload(
        &mut self,
    ) -> Option<Result<DeserializedMessage, ParseMessageError>> {
        let header_bytes = self.get_header_bytes()?;

        let Ok(header) = MessageHeader::deserialize_slice(&header_bytes) else {
            return Some(Err(ParseMessageError::InvalidHeader));
        };

        self.current_header = Some(header.clone());
        self.deserialize_payload_for(header)
    }

    fn get_header_bytes(&mut self) -> Option<[u8; MessageHeader::SERIALIZED_SIZE]> {
        if self.buffer.len() < MessageHeader::SERIALIZED_SIZE {
            return None;
        }
        let mut header_bytes = [0; MessageHeader::SERIALIZED_SIZE];

        match self.buffer.read_exact(&mut header_bytes) {
            Ok(_) => {}
            Err(_) => unreachable!("the buffer is big enough"),
        }
        Some(header_bytes)
    }

    fn deserialize_payload_for(
        &mut self,
        header: MessageHeader,
    ) -> Option<Result<DeserializedMessage, ParseMessageError>> {
        if self.buffer.len() < header.payload_length() {
            return None;
        }
        self.current_header = None;

        let digest = 0; // TODO

        // TODO: don't copy buffer
        let payload_buffer: Vec<u8> = self.buffer.drain(..header.payload_length()).collect();
        let Some(message) = Message::deserialize(&payload_buffer, &header, digest) else {
            return Some(Err(ParseMessageError::InvalidMessage(header.message_type)));
        };

        Some(Ok(DeserializedMessage::new(message, header.protocol)))
    }
}

impl Default for MessageDeserializerV2 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AscPullAck, Keepalive, MessageSerializer};

    mod happy_path {
        use super::*;

        #[test]
        fn default() {
            let deserializer = MessageDeserializerV2::default();
            assert_eq!(deserializer.buffer.capacity(), Message::MAX_MESSAGE_SIZE);
        }

        #[test]
        fn empty() {
            let mut deserializer = MessageDeserializerV2::new();
            assert!(deserializer.try_deserialize().is_none());
        }

        #[test]
        fn deserialize_message_without_payload() {
            let mut deserializer = MessageDeserializerV2::new();
            deserializer.push(&message_bytes(&Message::TelemetryReq));

            assert_eq!(
                deserializer.try_deserialize(),
                Some(Ok(DeserializedMessage::new(
                    Message::TelemetryReq,
                    Default::default()
                )))
            );
        }

        #[test]
        fn deserialize_message_with_payload() {
            let mut deserializer = MessageDeserializerV2::new();

            let keepalive = Message::Keepalive(Keepalive::new_test_instance());
            deserializer.push(&message_bytes(&keepalive));

            assert_eq!(
                deserializer.try_deserialize(),
                Some(Ok(DeserializedMessage::new(keepalive, Default::default())))
            );
        }

        #[test]
        fn deserialize_multiple_messages() {
            let mut deserializer = MessageDeserializerV2::new();

            let keepalive = Message::Keepalive(Keepalive::new_test_instance());
            deserializer.push(&message_bytes(&Message::TelemetryReq));
            deserializer.push(&message_bytes(&keepalive));
            deserializer.push(&message_bytes(&Message::TelemetryReq));

            assert_eq!(
                deserializer.try_deserialize(),
                Some(Ok(DeserializedMessage::new(
                    Message::TelemetryReq,
                    Default::default()
                )))
            );
            assert_eq!(
                deserializer.try_deserialize(),
                Some(Ok(DeserializedMessage::new(keepalive, Default::default())))
            );
            assert_eq!(
                deserializer.try_deserialize(),
                Some(Ok(DeserializedMessage::new(
                    Message::TelemetryReq,
                    Default::default()
                )))
            );
            assert_eq!(deserializer.try_deserialize(), None);
        }
    }

    mod unhappy_path {
        use crate::MessageType;

        use super::*;

        #[test]
        fn push_incomplete_header() {
            let mut deserializer = MessageDeserializerV2::new();
            deserializer.push(&[1, 2]);
            assert!(deserializer.try_deserialize().is_none());
        }

        #[test]
        fn invalid_header() {
            let mut deserializer = MessageDeserializerV2::new();
            deserializer.push(&[1, 2, 3, 4, 5, 6, 7, 8]);

            assert_eq!(
                deserializer.try_deserialize(),
                Some(Err(ParseMessageError::InvalidHeader))
            );
        }

        #[test]
        fn invalid_payload() {
            let mut deserializer = MessageDeserializerV2::new();
            deserializer.push(&invalid_asc_pull_ack_bytes());

            assert_eq!(
                deserializer.try_deserialize(),
                Some(Err(ParseMessageError::InvalidMessage(
                    MessageType::AscPullAck
                )))
            );
        }

        #[test]
        fn incomplete_playload() {
            let mut deserializer = MessageDeserializerV2::new();
            let keepalive = Message::Keepalive(Keepalive::new_test_instance());
            let data = message_bytes(&keepalive);

            // push incomplete message
            deserializer.push(&data[..data.len() - 1]);
            assert_eq!(deserializer.try_deserialize(), None);

            // push missing byte
            deserializer.push(&data[data.len() - 1..]);
            assert_eq!(
                deserializer.try_deserialize(),
                Some(Ok(DeserializedMessage::new(keepalive, Default::default())))
            );
        }
    }

    fn message_bytes(message: &Message) -> Vec<u8> {
        let mut serializer = MessageSerializer::default();
        serializer.serialize(message).to_vec()
    }

    fn invalid_asc_pull_ack_bytes() -> Vec<u8> {
        let mut data = message_bytes(&Message::AscPullAck(AscPullAck::new_test_instance_blocks()));
        // make message invalid:
        data[MessageHeader::SERIALIZED_SIZE..].fill(0xFF);
        data
    }
}
