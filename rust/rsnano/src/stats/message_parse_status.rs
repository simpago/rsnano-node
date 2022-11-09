use crate::transport::ParseStatus;

use super::DetailType;

impl From<ParseStatus> for DetailType {
    fn from(status: ParseStatus) -> Self {
        match status {
            ParseStatus::None | ParseStatus::Success => Self::All,
            ParseStatus::InsufficientWork => Self::InsufficientWork,
            ParseStatus::InvalidHeader => Self::InvalidHeader,
            ParseStatus::InvalidMessageType => Self::InvalidMessageType,
            ParseStatus::InvalidKeepaliveMessage => Self::InvalidKeepaliveMessage,
            ParseStatus::InvalidPublishMessage => Self::InvalidPublishMessage,
            ParseStatus::InvalidConfirmReqMessage => Self::InvalidConfirmReqMessage,
            ParseStatus::InvalidConfirmAckMessage => Self::InvalidConfirmAckMessage,
            ParseStatus::InvalidNodeIdHandshakeMessage => Self::InvalidNodeIdHandshakeMessage,
            ParseStatus::InvalidTelemetryReqMessage => Self::InvalidTelemetryReqMessage,
            ParseStatus::InvalidTelemetryAckMessage => Self::InvalidTelemetryAckMessage,
            ParseStatus::InvalidBulkPullMessage => Self::InvalidBulkPullMessage,
            ParseStatus::InvalidBulkPullAccountMessage => Self::InvalidBulkPullAccountMessage,
            ParseStatus::InvalidFrontierReqMessage => Self::InvalidFrontierReqMessage,
            ParseStatus::InvalidAscPullReqMessage => Self::InvalidAscPullReqMessage,
            ParseStatus::InvalidAscPullAckMessage => Self::InvalidAscPullAckMessage,
            ParseStatus::InvalidNetwork => Self::InvalidNetwork,
            ParseStatus::OutdatedVersion => Self::OutdatedVersion,
            ParseStatus::DuplicatePublishMessage => Self::DuplicatePublish,
            ParseStatus::MessageSizeTooBig => Self::MessageTooBig,
        }
    }
}
