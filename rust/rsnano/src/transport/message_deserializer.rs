use std::sync::{Arc, Mutex};

use crate::{
    config::NetworkConstants,
    core::{
        messages::{
            AscPullAck, AscPullReq, BulkPull, BulkPullAccount, BulkPush, ConfirmAck, ConfirmReq,
            FrontierReq, Keepalive, Message, MessageHeader, MessageType, NodeIdHandshake, Publish,
            TelemetryAck, TelemetryReq,
        },
        BlockUniquer,
    },
    transport::{Socket, SocketImpl},
    utils::{ErrorCode, Stream, StreamAdapter},
    voting::VoteUniquer,
};

use super::NetworkFilter;

const MAX_MESSAGE_SIZE: usize = 1024 * 4;
const HEADER_SIZE: usize = 8;

pub struct MessageDeserializer {
    network_constants: NetworkConstants,
    publish_filter: Arc<NetworkFilter>,
    block_uniquer: Arc<BlockUniquer>,
    vote_uniquer: Arc<VoteUniquer>,
    read_buffer: Arc<Mutex<Vec<u8>>>,
    status: Mutex<ParseStatus>,
}

impl MessageDeserializer {
    pub fn new(
        network_constants: NetworkConstants,
        network_filter: Arc<NetworkFilter>,
        block_uniquer: Arc<BlockUniquer>,
        vote_uniquer: Arc<VoteUniquer>,
    ) -> Self {
        Self {
            network_constants,
            publish_filter: network_filter,
            block_uniquer,
            vote_uniquer,
            status: Mutex::new(ParseStatus::None),
            read_buffer: Arc::new(Mutex::new(vec![0; MAX_MESSAGE_SIZE])),
        }
    }

    pub fn status(&self) -> ParseStatus {
        *self.status.lock().unwrap()
    }

    fn set_status(&self, status: ParseStatus) {
        let mut guard = self.status.lock().unwrap();
        *guard = status;
    }

    fn received_message(&self, header: MessageHeader, payload_size: usize, callback: CallbackType) {
        match self.deserialize(header, payload_size) {
            Some(message) => {
                debug_assert!(self.status() == ParseStatus::None);
                self.set_status(ParseStatus::Success);
                callback(ErrorCode::new(), Some(message));
            }
            None => {
                debug_assert!(self.status() != ParseStatus::None);
                callback(ErrorCode::new(), None);
            }
        }
    }

    fn deserialize(&self, header: MessageHeader, payload_size: usize) -> Option<Box<dyn Message>> {
        assert!(payload_size <= MAX_MESSAGE_SIZE);
        let buffer = self.read_buffer.lock().unwrap();
        let mut stream = StreamAdapter::new(&buffer[..payload_size]);
        match header.message_type() {
            MessageType::Keepalive => self.deserialize_keepalive(&mut stream, header),
            MessageType::Publish => {
                // Early filtering to not waste time deserializing duplicate blocks
                let (digest, existed) = self.publish_filter.apply(&buffer[..payload_size]);
                if !existed {
                    self.deserialize_publish(&mut stream, header, digest)
                } else {
                    self.set_status(ParseStatus::DuplicatePublishMessage);
                    None
                }
            }
            MessageType::ConfirmReq => self.deserialize_confirm_req(&mut stream, header),
            MessageType::ConfirmAck => self.deserialize_confirm_ack(&mut stream, header),
            MessageType::NodeIdHandshake => self.deserialize_node_id_handshake(&mut stream, header),
            MessageType::TelemetryReq => self.deserialize_telemetry_req(&mut stream, header),
            MessageType::TelemetryAck => self.deserialize_telemetry_ack(&mut stream, header),
            MessageType::BulkPull => self.deserialize_bulk_pull(&mut stream, header),
            MessageType::BulkPullAccount => self.deserialize_bulk_pull_account(&mut stream, header),
            MessageType::BulkPush => self.deserialize_bulk_push(&mut stream, header),
            MessageType::FrontierReq => self.deserialize_frontier_req(&mut stream, header),
            MessageType::AscPullReq => self.deserialize_asc_pull_req(&mut stream, header),
            MessageType::AscPullAck => self.deserialize_asc_pull_ack(&mut stream, header),
            MessageType::Invalid | MessageType::NotAType => {
                self.set_status(ParseStatus::InvalidMessageType);
                None
            }
        }
    }

    fn deserialize_keepalive(
        &self,
        stream: &mut impl Stream,
        header: MessageHeader,
    ) -> Option<Box<dyn Message>> {
        if let Ok(msg) = Keepalive::from_stream(header, stream) {
            if at_end(stream) {
                return Some(Box::new(msg));
            }
        }
        self.set_status(ParseStatus::InvalidKeepaliveMessage);
        None
    }

    fn deserialize_publish(
        &self,
        stream: &mut impl Stream,
        header: MessageHeader,
        digest: u128,
    ) -> Option<Box<dyn Message>> {
        if let Ok(msg) = Publish::from_stream(stream, header, digest, Some(&self.block_uniquer)) {
            if at_end(stream) {
                match &msg.block {
                    Some(block) => {
                        if !self
                            .network_constants
                            .work
                            .validate_entry_block(block.read().unwrap().as_block())
                        {
                            return Some(Box::new(msg));
                        } else {
                            self.set_status(ParseStatus::InsufficientWork);
                            return None;
                        }
                    }
                    None => unreachable!(), // successful deserialization always returns a block
                }
            }
        }

        self.set_status(ParseStatus::InvalidPublishMessage);
        None
    }

    fn deserialize_confirm_req(
        &self,
        stream: &mut impl Stream,
        header: MessageHeader,
    ) -> Option<Box<dyn Message>> {
        if let Ok(msg) = ConfirmReq::from_stream(stream, header, Some(&self.block_uniquer)) {
            if at_end(stream) {
                let work_ok = match msg.block() {
                    Some(block) => !self
                        .network_constants
                        .work
                        .validate_entry_block(block.read().unwrap().as_block()),
                    None => true,
                };
                if work_ok {
                    return Some(Box::new(msg));
                } else {
                    self.set_status(ParseStatus::InsufficientWork);
                    return None;
                }
            }
        }
        self.set_status(ParseStatus::InvalidConfirmReqMessage);
        None
    }

    fn deserialize_confirm_ack(
        &self,
        stream: &mut impl Stream,
        header: MessageHeader,
    ) -> Option<Box<dyn Message>> {
        if let Ok(msg) = ConfirmAck::with_header(header, stream, Some(&self.vote_uniquer)) {
            if at_end(stream) {
                return Some(Box::new(msg));
            }
        }
        self.set_status(ParseStatus::InvalidConfirmAckMessage);
        None
    }

    fn deserialize_node_id_handshake(
        &self,
        stream: &mut impl Stream,
        header: MessageHeader,
    ) -> Option<Box<dyn Message>> {
        if let Ok(msg) = NodeIdHandshake::from_stream(stream, header) {
            if at_end(stream) {
                return Some(Box::new(msg));
            }
        }

        self.set_status(ParseStatus::InvalidNodeIdHandshakeMessage);
        None
    }

    fn deserialize_telemetry_req(
        &self,
        _stream: &mut impl Stream,
        header: MessageHeader,
    ) -> Option<Box<dyn Message>> {
        // Message does not use stream payload (header only)
        Some(Box::new(TelemetryReq::with_header(header)))
    }

    fn deserialize_telemetry_ack(
        &self,
        stream: &mut impl Stream,
        header: MessageHeader,
    ) -> Option<Box<dyn Message>> {
        if let Ok(msg) = TelemetryAck::from_stream(stream, header) {
            // Intentionally not checking if at the end of stream, because these messages support backwards/forwards compatibility
            return Some(Box::new(msg));
        }
        self.set_status(ParseStatus::InvalidTelemetryAckMessage);
        None
    }

    fn deserialize_bulk_pull(
        &self,
        stream: &mut impl Stream,
        header: MessageHeader,
    ) -> Option<Box<dyn Message>> {
        if let Ok(msg) = BulkPull::from_stream(stream, header) {
            if at_end(stream) {
                return Some(Box::new(msg));
            }
        }
        self.set_status(ParseStatus::InvalidBulkPullMessage);
        None
    }

    fn deserialize_bulk_pull_account(
        &self,
        stream: &mut impl Stream,
        header: MessageHeader,
    ) -> Option<Box<dyn Message>> {
        if let Ok(msg) = BulkPullAccount::from_stream(stream, header) {
            if at_end(stream) {
                return Some(Box::new(msg));
            }
        }
        self.set_status(ParseStatus::InvalidBulkPullAccountMessage);
        None
    }

    fn deserialize_frontier_req(
        &self,
        stream: &mut impl Stream,
        header: MessageHeader,
    ) -> Option<Box<dyn Message>> {
        if let Ok(msg) = FrontierReq::from_stream(stream, header) {
            if at_end(stream) {
                return Some(Box::new(msg));
            }
        }
        self.set_status(ParseStatus::InvalidFrontierReqMessage);
        None
    }

    fn deserialize_bulk_push(
        &self,
        _stream: &mut impl Stream,
        header: MessageHeader,
    ) -> Option<Box<dyn Message>> {
        // Message does not use stream payload (header only)
        Some(Box::new(BulkPush::with_header(header)))
    }

    fn deserialize_asc_pull_req(
        &self,
        stream: &mut impl Stream,
        header: MessageHeader,
    ) -> Option<Box<dyn Message>> {
        // Intentionally not checking if at the end of stream, because these messages support backwards/forwards compatibility
        match AscPullReq::from_stream(stream, header) {
            Ok(msg) => Some(Box::new(msg)),
            Err(_) => {
                self.set_status(ParseStatus::InvalidAscPullReqMessage);
                None
            }
        }
    }

    fn deserialize_asc_pull_ack(
        &self,
        stream: &mut impl Stream,
        header: MessageHeader,
    ) -> Option<Box<dyn Message>> {
        // Intentionally not checking if at the end of stream, because these messages support backwards/forwards compatibility
        match AscPullAck::from_stream(stream, header) {
            Ok(msg) => Some(Box::new(msg)),
            Err(_) => {
                self.set_status(ParseStatus::InvalidAscPullAckMessage);
                None
            }
        }
    }
}

pub type CallbackType = Box<dyn FnOnce(ErrorCode, Option<Box<dyn Message>>)>;

pub trait MessageDeserializerExt {
    /// Asynchronously read next message from socket.
    /// If an irrecoverable error is encountered callback will be called with an error code set and `None` message.
    /// If a 'soft' error is encountered (eg. duplicate block publish) error won't be set but message will be `None`. In that case, `status` field will be set to code indicating reason for failure.
    /// If message is received successfully, error code won't be set and message will be non-null. `status` field will be set to `success`.
    /// Should not be called until the previous invocation finishes and calls the callback.
    fn read(&self, socket: Arc<SocketImpl>, callback: CallbackType);

    /// Deserializes message using data in `read_buffer`.
    /// # Return
    /// If successful returns non-null message, otherwise sets `status` to error appropriate code and returns `None`
    fn received_header(&self, socket: Arc<SocketImpl>, callback: CallbackType);
}

impl MessageDeserializerExt for Arc<MessageDeserializer> {
    fn read(&self, socket: Arc<SocketImpl>, callback: CallbackType) {
        self.set_status(ParseStatus::None);
        // Increase timeout to receive TCP header (idle server socket)
        let prev_timeout = socket.default_timeout_value();
        socket.set_default_timeout_value(self.network_constants.idle_timeout_s as u64);

        let self_clone = Arc::clone(self);
        let socket_clone = Arc::clone(&socket);
        socket.async_read2(
            Arc::clone(&self.read_buffer),
            HEADER_SIZE,
            Box::new(move |ec, size| {
                if ec.is_err() {
                    callback(ec, None);
                    return;
                }
                if size != HEADER_SIZE {
                    callback(ErrorCode::fault(), None);
                    return;
                }

                // Decrease timeout to default
                socket_clone.set_default_timeout_value(prev_timeout);

                self_clone.received_header(socket_clone, callback);
            }),
        );
    }

    fn received_header(&self, socket: Arc<SocketImpl>, callback: CallbackType) {
        let buffer = self.read_buffer.lock().unwrap();
        let mut stream = StreamAdapter::new(&buffer[..HEADER_SIZE]);
        let header = match MessageHeader::from_stream(&mut stream) {
            Ok(header) => header,
            Err(_) => {
                callback(ErrorCode::fault(), None);
                return;
            }
        };

        if header.network() != self.network_constants.current_network {
            self.set_status(ParseStatus::InvalidNetwork);
            callback(ErrorCode::fault(), None);
            return;
        }
        if header.version_using() < self.network_constants.protocol_version_min {
            self.set_status(ParseStatus::OutdatedVersion);
            callback(ErrorCode::fault(), None);
            return;
        }
        if !header.is_valid_message_type() {
            self.set_status(ParseStatus::InvalidHeader);
            callback(ErrorCode::fault(), None);
            return;
        }

        let payload_size = header.payload_length();
        if payload_size > MAX_MESSAGE_SIZE {
            self.set_status(ParseStatus::MessageSizeTooBig);
            callback(ErrorCode::fault(), None);
            return;
        }
        debug_assert!(payload_size <= buffer.len());
        drop(buffer);

        if payload_size == 0 {
            // Payload size will be 0 for `bulk_push` & `telemetry_req` message type
            self.received_message(header, 0, callback);
        } else {
            let self_clone = Arc::clone(self);
            socket.async_read2(
                Arc::clone(&self.read_buffer),
                payload_size,
                Box::new(move |ec, size| {
                    if ec.is_err() {
                        callback(ec, None);
                        return;
                    }
                    if size != payload_size {
                        callback(ErrorCode::fault(), None);
                        return;
                    }
                    self_clone.received_message(header, size, callback);
                }),
            );
        }
    }
}

#[derive(FromPrimitive, Clone, Copy, PartialEq, Eq)]
pub enum ParseStatus {
    None,
    Success,
    InsufficientWork,
    InvalidHeader,
    InvalidMessageType,
    InvalidKeepaliveMessage,
    InvalidPublishMessage,
    InvalidConfirmReqMessage,
    InvalidConfirmAckMessage,
    InvalidNodeIdHandshakeMessage,
    InvalidTelemetryReqMessage,
    InvalidTelemetryAckMessage,
    InvalidBulkPullMessage,
    InvalidBulkPullAccountMessage,
    InvalidFrontierReqMessage,
    InvalidAscPullReqMessage,
    InvalidAscPullAckMessage,
    InvalidNetwork,
    OutdatedVersion,
    DuplicatePublishMessage,
    MessageSizeTooBig,
}

impl ParseStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Success => "success",
            Self::InsufficientWork => "insufficient_work",
            Self::InvalidHeader => "invalid_header",
            Self::InvalidMessageType => "invalid_message_type",
            Self::InvalidKeepaliveMessage => "invalid_keepalive_message",
            Self::InvalidPublishMessage => "invalid_publish_message",
            Self::InvalidConfirmReqMessage => "invalid_confirm_req_message",
            Self::InvalidConfirmAckMessage => "invalid_confirm_ack_message",
            Self::InvalidNodeIdHandshakeMessage => "invalid_node_id_handshake_message",
            Self::InvalidTelemetryReqMessage => "invalid_telemetry_req_message",
            Self::InvalidTelemetryAckMessage => "invalid_telemetry_ack_message",
            Self::InvalidBulkPullMessage => "invalid_bulk_pull_message",
            Self::InvalidBulkPullAccountMessage => "invalid_bulk_pull_account_message",
            Self::InvalidFrontierReqMessage => "invalid_frontier_req_message",
            Self::InvalidAscPullReqMessage => "invalid_asc_pull_req_message",
            Self::InvalidAscPullAckMessage => "invalid_asc_pull_ack_message",
            Self::InvalidNetwork => "invalid_network",
            Self::OutdatedVersion => "outdated_version",
            Self::DuplicatePublishMessage => "duplicate_publish_message",
            Self::MessageSizeTooBig => "message_size_too_big",
        }
    }
}

fn at_end(stream: &mut impl Stream) -> bool {
    stream.read_u8().is_err()
}
