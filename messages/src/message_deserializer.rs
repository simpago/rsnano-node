use crate::{
    DeserializedMessage, Message, MessageHeader, MessageType, NetworkFilter, ParseMessageError,
};
use rsnano_core::work::WorkThresholds;
use std::{collections::VecDeque, io::Read, sync::Arc};

pub struct MessageDeserializerV2 {
    buffer: VecDeque<u8>,
    current_header: Option<MessageHeader>,
    network_filter: Arc<NetworkFilter>,
    work_thresholds: WorkThresholds,
}

impl MessageDeserializerV2 {
    pub fn new(network_filter: Arc<NetworkFilter>, work_thresholds: WorkThresholds) -> Self {
        Self {
            buffer: VecDeque::with_capacity(Message::MAX_MESSAGE_SIZE),
            current_header: None,
            network_filter,
            work_thresholds,
        }
    }

    pub fn push(&mut self, data: &[u8]) {
        self.buffer.extend(data);
    }

    pub fn try_deserialize(&mut self) -> Option<Result<DeserializedMessage, ParseMessageError>> {
        if let Some(header) = self.current_header.clone() {
            // The header was already deserialized. Only the payload has to be read:
            self.try_deserialize_payload_for(header)
        } else {
            self.deserialize_header_and_payload()
        }
    }

    fn deserialize_header_and_payload(
        &mut self,
    ) -> Option<Result<DeserializedMessage, ParseMessageError>> {
        let header_bytes = self.read_header_bytes()?;

        let Ok(header) = MessageHeader::deserialize_slice(&header_bytes) else {
            return Some(Err(ParseMessageError::InvalidHeader));
        };

        self.current_header = Some(header.clone());
        self.try_deserialize_payload_for(header)
    }

    fn read_header_bytes(&mut self) -> Option<[u8; MessageHeader::SERIALIZED_SIZE]> {
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

    fn try_deserialize_payload_for(
        &mut self,
        header: MessageHeader,
    ) -> Option<Result<DeserializedMessage, ParseMessageError>> {
        if self.buffer.len() < header.payload_length() {
            return None;
        }
        self.current_header = None;
        Some(self.deserialize_payload_for(header))
    }

    fn deserialize_payload_for(
        &mut self,
        header: MessageHeader,
    ) -> Result<DeserializedMessage, ParseMessageError> {
        let digest =
            self.filter_duplicate_messages(header.message_type, header.payload_length())?;

        // TODO: don't copy buffer
        let payload_buffer: Vec<u8> = self.buffer.drain(..header.payload_length()).collect();
        let Some(message) = Message::deserialize(&payload_buffer, &header, digest) else {
            return Err(ParseMessageError::InvalidMessage(header.message_type));
        };

        self.validate_work(&message)?;
        Ok(DeserializedMessage::new(message, header.protocol))
    }

    fn validate_work(&self, message: &Message) -> Result<(), ParseMessageError> {
        let block = match message {
            Message::Publish(msg) => Some(&msg.block),
            _ => None,
        };

        if let Some(block) = block {
            // work is checked multiple times - here and in the block processor and maybe
            // even more... TODO eliminate duplicate work checks
            if !self.work_thresholds.validate_entry_block(block) {
                return Err(ParseMessageError::InsufficientWork);
            }
        }

        Ok(())
    }

    /// Early filtering to not waste time deserializing duplicate blocks
    fn filter_duplicate_messages(
        &self,
        message_type: MessageType,
        payload_len: usize,
    ) -> Result<u128, ParseMessageError> {
        if matches!(message_type, MessageType::Publish | MessageType::ConfirmAck) {
            // TODO: don't copy payload bytes'
            let payload_bytes: Vec<u8> = self.buffer.iter().cloned().take(payload_len).collect();
            let (digest, existed) = self.network_filter.apply(&payload_bytes);
            if existed {
                if message_type == MessageType::ConfirmAck {
                    Err(ParseMessageError::DuplicateConfirmAckMessage)
                } else {
                    Err(ParseMessageError::DuplicatePublishMessage)
                }
            } else {
                Ok(digest)
            }
        } else {
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AscPullAck, Keepalive, MessageSerializer};

    mod happy_path {
        use super::*;

        #[test]
        fn empty() {
            let mut deserializer = create_deserializer();
            assert!(deserializer.try_deserialize().is_none());
        }

        #[test]
        fn deserialize_message_without_payload() {
            let mut deserializer = create_deserializer();
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
            let mut deserializer = create_deserializer();

            let keepalive = Message::Keepalive(Keepalive::new_test_instance());
            deserializer.push(&message_bytes(&keepalive));

            assert_eq!(
                deserializer.try_deserialize(),
                Some(Ok(DeserializedMessage::new(keepalive, Default::default())))
            );
        }

        #[test]
        fn deserialize_multiple_messages() {
            let mut deserializer = create_deserializer();

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
        use crate::{ConfirmAck, MessageType, Publish};

        use super::*;

        #[test]
        fn push_incomplete_header() {
            let mut deserializer = create_deserializer();
            deserializer.push(&[1, 2]);
            assert!(deserializer.try_deserialize().is_none());
        }

        #[test]
        fn invalid_header() {
            let mut deserializer = create_deserializer();
            deserializer.push(&[1, 2, 3, 4, 5, 6, 7, 8]);

            assert_eq!(
                deserializer.try_deserialize(),
                Some(Err(ParseMessageError::InvalidHeader))
            );
        }

        #[test]
        fn invalid_payload() {
            let mut deserializer = create_deserializer();
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
            let mut deserializer = create_deserializer();
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

        #[test]
        fn insufficient_work() {
            let mut deserializer = create_deserializer();

            let mut publish = Publish::new_test_instance();
            publish.block.set_work(0);
            let message = Message::Publish(publish);

            deserializer.push(&message_bytes(&message));
            let result = deserializer.try_deserialize().unwrap();

            assert_eq!(result, Err(ParseMessageError::InsufficientWork));
        }

        // Send two publish messages and asserts that the duplication is detected.
        #[test]
        fn duplicate_publish_message() {
            let mut deserializer = create_deserializer();

            let message = Message::Publish(Publish::new_test_instance());
            let message_bytes = message_bytes(&message);

            deserializer.push(&message_bytes);
            deserializer.try_deserialize();

            deserializer.push(&message_bytes);
            let result = deserializer.try_deserialize();

            assert_eq!(
                result,
                Some(Err(ParseMessageError::DuplicatePublishMessage))
            );
        }
        //
        // Send two publish messages and asserts that the duplication is detected.
        #[test]
        fn duplicate_confirm_ack() {
            let mut deserializer = create_deserializer();
            let message = Message::ConfirmAck(ConfirmAck::new_test_instance());
            let message_bytes = message_bytes(&message);

            deserializer.push(&message_bytes);
            deserializer.try_deserialize();

            deserializer.push(&message_bytes);
            let result = deserializer.try_deserialize();

            assert_eq!(
                result,
                Some(Err(ParseMessageError::DuplicateConfirmAckMessage))
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

    fn create_deserializer() -> MessageDeserializerV2 {
        let filter = Arc::new(NetworkFilter::default());
        MessageDeserializerV2::new(filter, WorkThresholds::publish_dev().clone())
    }
}
