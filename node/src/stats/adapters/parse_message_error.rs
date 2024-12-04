use super::DetailType;
use rsnano_messages::{MessageType, ParseMessageError};

impl From<&ParseMessageError> for DetailType {
    fn from(status: &ParseMessageError) -> Self {
        match status {
            ParseMessageError::Other(_) | ParseMessageError::Stopped => Self::All,
            ParseMessageError::InsufficientWork => Self::InsufficientWork,
            ParseMessageError::InvalidHeader => Self::InvalidHeader,
            ParseMessageError::InvalidMessageType => Self::InvalidMessageType,
            ParseMessageError::InvalidMessage(MessageType::Keepalive) => {
                Self::InvalidKeepaliveMessage
            }
            ParseMessageError::InvalidMessage(MessageType::Publish) => Self::InvalidPublishMessage,
            ParseMessageError::InvalidMessage(MessageType::ConfirmReq) => {
                Self::InvalidConfirmReqMessage
            }
            ParseMessageError::InvalidMessage(MessageType::ConfirmAck) => {
                Self::InvalidConfirmAckMessage
            }
            ParseMessageError::InvalidMessage(MessageType::NodeIdHandshake) => {
                Self::InvalidNodeIdHandshakeMessage
            }
            ParseMessageError::InvalidMessage(MessageType::TelemetryReq) => {
                Self::InvalidTelemetryReqMessage
            }
            ParseMessageError::InvalidMessage(MessageType::TelemetryAck) => {
                Self::InvalidTelemetryAckMessage
            }
            ParseMessageError::InvalidMessage(MessageType::BulkPull) => {
                Self::InvalidBulkPullMessage
            }
            ParseMessageError::InvalidMessage(MessageType::BulkPullAccount) => {
                Self::InvalidBulkPullAccountMessage
            }
            ParseMessageError::InvalidMessage(MessageType::FrontierReq) => {
                Self::InvalidFrontierReqMessage
            }
            ParseMessageError::InvalidMessage(MessageType::AscPullReq) => {
                Self::InvalidAscPullReqMessage
            }
            ParseMessageError::InvalidMessage(MessageType::AscPullAck) => {
                Self::InvalidAscPullAckMessage
            }
            ParseMessageError::InvalidMessage(MessageType::BulkPush) => Self::InvalidMessageType,
            ParseMessageError::InvalidMessage(MessageType::Invalid)
            | ParseMessageError::InvalidMessage(MessageType::NotAType) => Self::InvalidMessageType,
            ParseMessageError::InvalidNetwork => Self::InvalidNetwork,
            ParseMessageError::OutdatedVersion => Self::OutdatedVersion,
            ParseMessageError::DuplicatePublishMessage => Self::DuplicatePublishMessage,
            ParseMessageError::DuplicateConfirmAckMessage => Self::DuplicateConfirmAckMessage,
            ParseMessageError::MessageSizeTooBig => Self::MessageSizeTooBig,
        }
    }
}
