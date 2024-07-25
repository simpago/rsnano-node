use super::{NetworkFilter, Socket};
use async_trait::async_trait;
use rsnano_core::{utils::BufferReader, work::WorkThresholds};
use rsnano_messages::*;
use std::sync::{Arc, Mutex};

#[async_trait]
pub trait AsyncBufferReader {
    async fn read(&self, buffer: Arc<Mutex<Vec<u8>>>, count: usize) -> anyhow::Result<()>;
}

pub struct MessageDeserializer<T: AsyncBufferReader + Send> {
    network_filter: Arc<NetworkFilter>,
    work_thresholds: WorkThresholds,
    protocol_info: ProtocolInfo,
    read_buffer: Arc<Mutex<Vec<u8>>>,
    buffer_reader: T,
}

impl MessageDeserializer<Arc<Socket>> {
    pub fn new_null() -> Self {
        Self {
            network_filter: Arc::new(NetworkFilter::default()),
            work_thresholds: WorkThresholds::new(0, 0, 0),
            protocol_info: ProtocolInfo::default(),
            read_buffer: Arc::new(Mutex::new(Vec::new())),
            buffer_reader: Socket::new_null(),
        }
    }
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
            read_buffer: Arc::new(Mutex::new(vec![0; Message::MAX_MESSAGE_SIZE])),
            buffer_reader,
            work_thresholds,
            network_filter,
        }
    }

    pub async fn read(&self) -> Result<DeserializedMessage, ParseMessageError> {
        self.buffer_reader
            .read(
                Arc::clone(&self.read_buffer),
                MessageHeader::SERIALIZED_SIZE,
            )
            .await
            .map_err(|e| ParseMessageError::Other(e.to_string()))?;

        self.received_header().await
    }

    async fn received_header(&self) -> Result<DeserializedMessage, ParseMessageError> {
        let header = {
            let buffer = self.read_buffer.lock().unwrap();
            let header_bytes = &buffer[..MessageHeader::SERIALIZED_SIZE];
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
                .read(Arc::clone(&self.read_buffer), payload_size)
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
        let buffer = self.read_buffer.lock().unwrap();
        let payload_bytes = &buffer[..payload_size];
        let digest = self.filter_duplicate_publish_messages(header.message_type, payload_bytes)?;
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
            if self.work_thresholds.validate_entry_block(block) {
                return Err(ParseMessageError::InsufficientWork);
            }
        }

        Ok(())
    }

    fn filter_duplicate_publish_messages(
        &self,
        message_type: MessageType,
        payload_bytes: &[u8],
    ) -> Result<u128, ParseMessageError> {
        if message_type == MessageType::Publish {
            // Early filtering to not waste time deserializing duplicate blocks
            let (digest, existed) = self.network_filter.apply(payload_bytes);
            if existed {
                Err(ParseMessageError::DuplicatePublishMessage)
            } else {
                Ok(digest)
            }
        } else {
            Ok(0)
        }
    }
}
