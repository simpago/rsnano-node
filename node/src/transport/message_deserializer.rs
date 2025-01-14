use super::NetworkFilter;
use rsnano_core::{utils::BufferReader, work::WorkThresholds};
use rsnano_messages::*;
use rsnano_network::AsyncBufferReader;
use std::sync::Arc;

pub struct MessageDeserializer<T: AsyncBufferReader + Send> {
    network_filter: Arc<NetworkFilter>,
    work_thresholds: WorkThresholds,
    protocol_info: ProtocolInfo,
    read_buffer: Vec<u8>,
    buffer_reader: T,
}

impl<T: AsyncBufferReader + Send> MessageDeserializer<T> {
    pub fn new(
        protocol_info: ProtocolInfo,
        work_thresholds: WorkThresholds,
        network_filter: Arc<NetworkFilter>,
        buffer_reader: T,
    ) -> Self {
        Self {
            protocol_info,
            read_buffer: vec![0; Message::MAX_MESSAGE_SIZE],
            buffer_reader,
            work_thresholds,
            network_filter,
        }
    }

    pub async fn read(&mut self) -> Result<DeserializedMessage, ParseMessageError> {
        self.buffer_reader
            .read(&mut self.read_buffer, MessageHeader::SERIALIZED_SIZE)
            .await
            .map_err(|e| ParseMessageError::Other(e.to_string()))?;

        self.received_header().await
    }

    async fn received_header(&mut self) -> Result<DeserializedMessage, ParseMessageError> {
        let header = {
            let header_bytes = &self.read_buffer[..MessageHeader::SERIALIZED_SIZE];
            let mut stream = BufferReader::new(header_bytes);
            MessageHeader::deserialize(&mut stream).map_err(|_| ParseMessageError::InvalidHeader)?
        };

        validate_header(&header, &self.protocol_info)?;
        let payload_size = header.payload_length();
        if payload_size == 0 {
            // Payload size will be 0 for `bulk_push` & `telemetry_req` message type
            self.parse_message(header, 0)
        } else {
            self.buffer_reader
                .read(&mut self.read_buffer, payload_size)
                .await
                .map_err(|e| ParseMessageError::Other(e.to_string()))?;

            self.parse_message(header, payload_size)
        }
    }

    fn parse_message(
        &self,
        header: MessageHeader,
        payload_size: usize,
    ) -> Result<DeserializedMessage, ParseMessageError> {
        let payload_bytes = &self.read_buffer[..payload_size];
        let digest = self.filter_duplicate_messages(header.message_type, payload_bytes)?;
        let message = Message::deserialize(payload_bytes, &header, digest)
            .ok_or(ParseMessageError::InvalidMessage(header.message_type))?;
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
        payload_bytes: &[u8],
    ) -> Result<u128, ParseMessageError> {
        if matches!(message_type, MessageType::Publish | MessageType::ConfirmAck) {
            let (digest, existed) = self.network_filter.apply(payload_bytes);
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
    use crate::transport::VecBufferReader;

    #[tokio::test]
    async fn insufficient_work() {
        let protocol = ProtocolInfo::default();
        let mut publish = Publish::new_test_instance();
        publish.block.set_work(0);
        let message = Message::Publish(publish);
        let mut serializer = MessageSerializer::new(protocol);
        let buffer = serializer.serialize(&message).to_vec();
        let reader = VecBufferReader::new(buffer);

        let mut deserializer = MessageDeserializer::new(
            protocol,
            WorkThresholds::publish_full().clone(),
            Arc::new(NetworkFilter::default()),
            reader,
        );

        let error = deserializer.read().await.unwrap_err();

        assert_eq!(error, ParseMessageError::InsufficientWork);
    }

    // Send two publish messages and asserts that the duplication is detected.
    #[tokio::test]
    async fn duplicate_publish_message() {
        let protocol = ProtocolInfo::default();
        let message = Message::Publish(Publish::new_test_instance());
        let mut serializer = MessageSerializer::new(protocol);
        let mut buffer = serializer.serialize(&message).to_vec();
        buffer.extend_from_slice(serializer.serialize(&message));
        let reader = VecBufferReader::new(buffer);

        let mut deserializer = MessageDeserializer::new(
            protocol,
            WorkThresholds::new(0, 0, 0),
            Arc::new(NetworkFilter::default()),
            reader,
        );

        deserializer.read().await.unwrap();
        let error = deserializer.read().await.unwrap_err();

        assert_eq!(error, ParseMessageError::DuplicatePublishMessage);
    }

    // Send two publish messages and asserts that the duplication is detected.
    #[tokio::test]
    async fn duplicate_confirm_ack() {
        let protocol = ProtocolInfo::default();
        let message = Message::ConfirmAck(ConfirmAck::new_test_instance());
        let mut serializer = MessageSerializer::new(protocol);
        let mut buffer = serializer.serialize(&message).to_vec();
        buffer.extend_from_slice(serializer.serialize(&message));
        let reader = VecBufferReader::new(buffer);

        let mut deserializer = MessageDeserializer::new(
            protocol,
            WorkThresholds::new(0, 0, 0),
            Arc::new(NetworkFilter::default()),
            reader,
        );

        deserializer.read().await.unwrap();
        let error = deserializer.read().await.unwrap_err();

        assert_eq!(error, ParseMessageError::DuplicateConfirmAckMessage);
    }
}
